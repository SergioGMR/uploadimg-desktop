import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const html = await readFile(new URL('../index.html', import.meta.url), 'utf8');
const candidateId = 'a'.repeat(32), resultId = 'b'.repeat(48);
const state = overrides => ({ simulation: false, nativeAvailable: true, platform: 'macos', paused: false,
  clipboardEnabled: false, clipboardAvailable: true, folderAvailable: false, notificationStatus: 'not_requested',
  status: 'Listo', selectedCandidateId: null, selectedResultId: null, pending: [], results: [], upload: null, ...overrides });
const candidate = { id: candidateId, source: 'synthetic', width: 10, height: 10, inputBytes: 100, expiresInSeconds: 60, previewDataUrl: '' };
const flush = () => new Promise(resolve => setImmediate(resolve));

// Small DOM fixture for event/control behavior; it is not browser rendering evidence.
class Node {
  constructor(tag = 'div') {
    this.tagName = tag; this.children = []; this.dataset = {}; this.attributes = new Map();
    this.listeners = new Map(); this.textContent = ''; this.hidden = false; this.disabled = false;
    this.classList = { toggle: () => {} };
  }
  append(...nodes) { for (const node of nodes) this.insertBefore(node, null); }
  insertBefore(node, sibling) {
    node.remove();
    const index = sibling ? this.children.indexOf(sibling) : this.children.length;
    this.children.splice(index, 0, node); node.parent = this;
  }
  remove() { if (this.parent) { this.parent.children.splice(this.parent.children.indexOf(this), 1); this.parent = null; } }
  setAttribute(key, value) { this.attributes.set(key, value); }
  getAttribute(key) { return this.attributes.get(key) ?? null; }
  removeAttribute(key) { this.attributes.delete(key); }
  addEventListener(event, handler) { this.listeners.set(event, handler); }
  async click() { if (!this.disabled) await this.listeners.get('click')?.(); }
}
let imports = 0;
async function setup(initial) {
  const nodes = Object.fromEntries([...html.matchAll(/id="([^"]+)"/g)].map(([, id]) => [id, new Node()]));
  const calls = [];
  let current = initial, poll;
  const walk = node => [node, ...node.children.flatMap(walk)];
  globalThis.document = { getElementById: id => nodes[id], createElement: tag => new Node(tag),
    querySelectorAll: () => Object.values(nodes).flatMap(walk).filter(node => node.dataset.command) };
  globalThis.window = { __TAURI__: { core: { invoke: async (command, args) => { calls.push([command, args]); return current; } } },
    setInterval: callback => { poll = callback; return 1; }, clearInterval() {}, addEventListener() {} };
  await import(`../app.mjs?test=${++imports}`);
  await flush();
  return { nodes, calls, async update(value) { current = value; poll(); await flush(); },
    buttons(id) { return walk(nodes[id]).filter(node => node.tagName === 'button'); } };
}

test('visible brand and consent explain the service and temporary public links', () => {
  assert.match(html, /<title>UploadIMG<\/title>/);
  assert.match(html, /<h1>UploadIMG<\/h1>/);
  assert.match(html, /viewBox="0 0 24 24"/);
  assert.match(html, /Compartir[^<]*envía[^<]*uploadimg\.vercel\.app/);
  assert.match(html, /24 horas/);
  assert.doesNotMatch(html, /simulad|fictici|No se suben/i);
});

test('sharing is click-only and a live upload disables every candidate Share', async () => {
  const app = await setup(state({ pending: [candidate] }));
  const share = app.buttons('pending-list').find(button => button.textContent === 'Compartir');
  assert.ok(share, 'the real Share action must be visible');
  assert.deepEqual(app.calls, [['get_state', {}]], 'rendering a candidate never starts an upload');
  await share.click();
  assert.deepEqual(app.calls.at(-1), ['candidate_action', { id: candidateId, action: 'share' }]);
  await app.update(state({ pending: [candidate], upload: { id: 'c'.repeat(32), status: 'uploading' } }));
  assert.equal(share.disabled, true);
  assert.equal(app.nodes['upload-panel'].hidden, false);
  assert.match(app.nodes['upload-status'].textContent, /curso/);
  assert.equal(app.nodes['recover-upload'].hidden, true);
  const before = app.calls.length;
  await share.click();
  assert.equal(app.calls.length, before);
});

test('recovery works without a pending candidate and invokes the same opaque share ID', async () => {
  const app = await setup(state({ upload: { id: candidateId, status: 'recoverable' } }));
  const recovery = app.nodes['recover-upload'];
  assert.equal(recovery.hidden, false);
  assert.equal(recovery.disabled, false);
  assert.match(app.nodes['upload-status'].textContent, /sin volver a enviar/);
  assert.match(app.nodes['upload-status'].textContent, /No se pudo confirmar/);
  assert.doesNotMatch(app.nodes['upload-status'].textContent, /La imagen se envió/);
  await recovery.click();
  assert.deepEqual(app.calls.at(-1), ['candidate_action', { id: candidateId, action: 'share' }]);
  await app.update(state({ upload: { id: candidateId, status: 'uploading' } }));
  assert.equal(recovery.hidden, true);
  assert.equal(recovery.disabled, true);
  await app.update(state({ results: [{ id: resultId, url: `https://uploadimg.vercel.app/i/${resultId}`, fictitious: false, expiresInSeconds: 86400 }] }));
  assert.equal(app.nodes['upload-panel'].hidden, true);
  assert.equal(app.nodes['results-empty'].hidden, true);
  const result = app.nodes['results-list'].children[0];
  assert.equal(result.children[0].children[0].textContent, `https://uploadimg.vercel.app/i/${resultId}`);
  assert.equal(result.children[0].children[1].textContent, 'Caduca en 24 h');
  await app.buttons('results-list').find(button => button.textContent === 'Abrir enlace').click();
  assert.deepEqual(app.calls.at(-1), ['result_action', { id: resultId, action: 'open' }]);
});
