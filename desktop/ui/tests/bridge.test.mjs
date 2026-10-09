import test from 'node:test';
import assert from 'node:assert/strict';
import { deflateSync } from 'node:zlib';
import { createBridge, commandArguments, previewSource } from '../bridge.mjs';

const state = (status) => ({ simulation: false, status });
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};

function pngChunk(type, contents) {
  const name = Buffer.from(type);
  const length = Buffer.alloc(4), checksum = Buffer.alloc(4);
  length.writeUInt32BE(contents.length);
  let crc = 0xffffffff;
  for (const byte of Buffer.concat([name, contents])) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
  }
  checksum.writeUInt32BE((crc ^ 0xffffffff) >>> 0);
  return Buffer.concat([length, name, contents, checksum]);
}

// A real 512×512 RGBA PNG, uncompressed so its base64 exceeds 1.1M characters.
function largePng(byteLength) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(512, 0); header.writeUInt32BE(512, 4);
  header[8] = 8; header[9] = 6;
  const chunks = [Buffer.from('89504e470d0a1a0a', 'hex'), pngChunk('IHDR', header),
    pngChunk('IDAT', deflateSync(Buffer.alloc(512 * (512 * 4 + 1)), { level: 0 }))];
  const end = pngChunk('IEND', Buffer.alloc(0));
  const paddingLength = byteLength - Buffer.concat([...chunks, end]).length - 12;
  const note = Buffer.alloc(paddingLength, 97);
  note.write('note\0');
  return Buffer.concat([...chunks, pngChunk('tEXt', note), end]);
}

test('only the documented commands and narrow arguments reach native code', () => {
  assert.deepEqual(commandArguments('candidate_action', { id: 'a'.repeat(32), action: 'share' }), { id: 'a'.repeat(32), action: 'share' });
  assert.deepEqual(commandArguments('set_clipboard_enabled', { enabled: false }), { enabled: false });
  for (const [command, args] of [
    ['read_path', {}], ['paste_image', { path: '/private/image.png' }],
    ['result_action', { id: 'b'.repeat(48), action: 'open', url: 'https://evil.invalid' }],
    ['candidate_action', { id: '', action: 'share' }],
    ['candidate_action', { id: 'not-opaque', action: 'share' }],
    ['result_action', { id: 'a'.repeat(32), action: 'copy' }],
    ['candidate_action', { id: 'a'.repeat(32), action: 'copy' }],
    ['set_clipboard_enabled', { enabled: 'true' }],
  ]) assert.throws(() => commandArguments(command, args));
});

test('ordinary browser cannot fabricate native state or action success', async () => {
  const seen = [];
  const bridge = createBridge({ onState: value => seen.push(value) });
  assert.equal(bridge.available, false);
  assert.equal(await bridge.poll(), false);
  await assert.rejects(bridge.act('generate_synthetic'), /UploadIMG/);
  assert.deepEqual(seen, []);
});

test('an in-flight poll cannot revert an action, and invokes never overlap', async () => {
  const first = deferred();
  const second = deferred();
  const calls = [], seen = [], busy = [];
  const bridge = createBridge({ invoke: (command, args) => {
    calls.push([command, args]);
    return calls.length === 1 ? first.promise : second.promise;
  }, onState: value => seen.push(value), onBusy: value => busy.push(value) });
  const poll = bridge.poll();
  await Promise.resolve();
  const action = bridge.act('set_clipboard_enabled', { enabled: true });
  assert.equal(await bridge.poll(), false);
  assert.equal(calls.length, 1);
  first.resolve(state('old'));
  await poll;
  await Promise.resolve();
  assert.deepEqual(seen, []);
  assert.deepEqual(calls, [['get_state', {}], ['set_clipboard_enabled', { enabled: true }]]);
  second.resolve(state('enabled'));
  await action;
  assert.deepEqual(seen, [state('enabled')]);
  assert.deepEqual(busy, [true, false]);
});

test('failed actions preserve confirmed state and expose the error', async () => {
  const seen = [], errors = [];
  const bridge = createBridge({ invoke: async command => {
    if (command === 'get_state') return state('confirmed');
    throw new Error('Permiso denegado');
  }, onState: value => seen.push(value), onError: value => errors.push(value) });
  await bridge.poll();
  await assert.rejects(bridge.act('paste_image'), /Permiso denegado/);
  assert.deepEqual(seen, [state('confirmed')]);
  assert.equal(errors.at(-1), 'Permiso denegado');
  await bridge.poll();
  assert.equal(errors.at(-1), 'Permiso denegado', 'a timer poll must not erase the action error');
});

test('polls coalesce, and queued actions retain order after a failure', async () => {
  const pending = deferred();
  const calls = [];
  const bridge = createBridge({ invoke: async command => {
    calls.push(command);
    if (command === 'get_state') return pending.promise;
    if (command === 'paste_image') throw new Error('Sin imagen');
    return state('test image');
  } });
  const poll = bridge.poll();
  assert.equal(await bridge.poll(), false);
  const paste = bridge.act('paste_image');
  const generate = bridge.act('generate_synthetic');
  pending.resolve(state('old'));
  await poll;
  await assert.rejects(paste, /Sin imagen/);
  await generate;
  assert.deepEqual(calls, ['get_state', 'paste_image', 'generate_synthetic']);
});

test('thumbnails require bounded PNG dimensions and exact data MIME', () => {
  const png = Buffer.from('89504e470d0a1a0a0000000d4948445200000001000000010806000000', 'hex');
  const url = `data:image/png;base64,${png.toString('base64')}`;
  assert.equal(previewSource(url), url);
  assert.equal(previewSource('data:image/svg+xml;base64,PHN2Zz4='), '');
  assert.equal(previewSource('https://example.invalid/image.png'), '');
  assert.equal(previewSource('data:image/png;base64,not-valid!'), '');
  const oversized = Buffer.from(png);
  oversized.writeUInt32BE(513, 16);
  assert.equal(previewSource(`data:image/png;base64,${oversized.toString('base64')}`), '');
  assert.equal(previewSource(`data:image/png;base64,${'A'.repeat(1_100_001)}`), '');
});

test('PNG byte cap accepts base64 expansion at the exact limit and rejects one extra byte', () => {
  const atLimit = largePng(1_100_000);
  const accepted = `data:image/png;base64,${atLimit.toString('base64')}`;
  assert.equal(atLimit.length, 1_100_000);
  assert.equal(accepted.length, 1_466_690);
  assert.ok(accepted.length > 1_100_000);
  assert.ok(previewSource(accepted) === accepted, 'a valid PNG at the byte limit must be accepted');
  const belowLimit = `data:image/png;base64,${largePng(1_099_999).toString('base64')}`;
  assert.ok(previewSource(belowLimit) === belowLimit, 'two-character padding must retain its correct byte count');
  const overLimit = largePng(1_100_001);
  const rejected = `data:image/png;base64,${overLimit.toString('base64')}`;
  assert.equal(rejected.length, accepted.length, 'padding changes decoded size without increasing encoded size');
  assert.equal(previewSource(rejected), '');
});
