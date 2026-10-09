import test from 'node:test';
import assert from 'node:assert/strict';
import { readUiState } from '../state.mjs';

const candidateId = 'a'.repeat(32), resultId = 'b'.repeat(48);
const fixture = () => ({
  simulation: false, nativeAvailable: true, platform: 'macos', paused: false,
  clipboardEnabled: false, clipboardAvailable: true, folderAvailable: false,
  notificationStatus: 'not_requested', status: 'Listo', pending: [], results: [],
  selectedCandidateId: null, selectedResultId: null, upload: null,
});
const result = () => ({ id: resultId, url: `https://uploadimg.vercel.app/i/${resultId}`, fictitious: false, expiresInSeconds: 86400 });

test('accepts the real native contract and ignores additive fields', () => {
  const value = fixture();
  assert.deepEqual(readUiState({ ...value, extra: 'ignored' }), value);
});

test('rejects incompatible, unbounded or misleading native summaries', () => {
  for (const change of [
    { simulation: true }, { paused: 'false' }, { folderAvailable: true },
    { notificationStatus: 'perhaps' }, { pending: new Array(4).fill({}) },
    { results: new Array(21).fill({}) }, { status: 'x'.repeat(501) },
    { results: [{ ...result(), fictitious: true }] },
    { results: [{ ...result(), expiresInSeconds: 86401 }] },
    { pending: [{ id: candidateId, source: 'file', width: 1, height: 1, inputBytes: 1, expiresInSeconds: 60, previewDataUrl: '' }] },
    { selectedCandidateId: 'not-opaque' }, { selectedResultId: candidateId },
  ]) assert.throws(() => readUiState({ ...fixture(), ...change }), /Estado/);
});

test('only the exact HTTPS result route matching the opaque result ID is accepted', () => {
  const valid = result();
  assert.deepEqual(readUiState({ ...fixture(), results: [valid] }).results, [valid]);
  for (const url of [
    `http://uploadimg.vercel.app/i/${resultId}`, `https://evil.invalid/i/${resultId}`,
    `https://uploadimg.vercel.app.evil.invalid/i/${resultId}`,
    `https://user@uploadimg.vercel.app/i/${resultId}`, `https://uploadimg.vercel.app:443/i/${resultId}`,
    `${valid.url}?other=true`, `${valid.url}#fragment`, `${valid.url}/`,
    `https://uploadimg.vercel.app/i/${'c'.repeat(48)}`, `https://uploadimg.vercel.app/i/${'B'.repeat(48)}`,
    'https://example.invalid/i/result', 'javascript:alert(1)',
  ]) assert.throws(() => readUiState({ ...fixture(), results: [{ ...valid, url }] }), /Estado/, url);
});

test('accepts only bounded upload IDs and known lifecycle states', () => {
  for (const status of ['uploading', 'recoverable']) {
    const upload = { id: candidateId, status };
    assert.deepEqual(readUiState({ ...fixture(), upload }).upload, upload);
  }
  for (const upload of [undefined, {}, { id: resultId, status: 'uploading' },
    { id: 'A'.repeat(32), status: 'recoverable' }, { id: candidateId, status: 'complete' }]) {
    assert.throws(() => readUiState({ ...fixture(), upload }), /Estado/);
  }
});

test('does not display unsafe thumbnails and retains only safe result text', () => {
  const candidate = { id: candidateId, source: 'clipboard', width: 20, height: 30, inputBytes: 40, expiresInSeconds: 60, previewDataUrl: 'data:image/svg+xml;base64,PHN2Zz4=' };
  const value = readUiState({ ...fixture(), pending: [candidate], results: [result()] });
  assert.equal(value.pending[0].previewDataUrl, '');
  assert.equal(value.results[0].url, result().url);
});
