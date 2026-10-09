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
  removeEventListener(event, handler) { if (this.listeners.get(event) === handler) this.listeners.delete(event); }
  async click() { if (!this.disabled) await this.listeners.get('click')?.(); }
}
let imports = 0;
async function setup(initial) {
  const nodes = Object.fromEntries([...html.matchAll(/id="([^"]+)"/g)].map(([, id]) => [id, new Node()]));
  const calls = [];
  const updateStatus = { state: 'disabled', currentVersion: '0.1.0-beta.2', offeredVersion: null,
    channel: 'beta', message: 'Configuración pendiente.', recoveryPath: null, canInstall: false, blockedReason: null };
  let current = initial, poll;
  const walk = node => [node, ...node.children.flatMap(walk)];
  globalThis.document = { getElementById: id => nodes[id], createElement: tag => new Node(tag),
    querySelectorAll: () => Object.values(nodes).flatMap(walk).filter(node => node.dataset.command) };
  globalThis.window = { __TAURI__: { core: { invoke: async (command, args) => { calls.push([command, args]); return command.startsWith('updater_') ? updateStatus : current; } } },
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

test('update panel is wired to read-only native status and keeps the existing image controls', async () => {
  assert.match(html, /id="update-check"[^>]*>Buscar actualizaciones</);
  assert.match(html, /id="update-install"[^>]*>Instalar y reiniciar</);
  const app = await setup(state());
  assert.equal(app.calls.filter(([command]) => command === 'updater_status').length, 1);
  assert.equal(app.calls.filter(([command]) => command === 'updater_check').length, 0);
  assert.equal(app.nodes.generate.disabled, false);
  assert.equal(app.nodes.clipboard.checked, false);
  assert.equal(app.nodes['update-version'].textContent, 'Versión actual: 0.1.0-beta.2');
  assert.equal(app.nodes['update-check'].disabled, true);
});

test('sharing is click-only and a live upload disables every candidate Share', async () => {
  const app = await setup(state({ pending: [candidate] }));
  const share = app.buttons('pending-list').find(button => button.textContent === 'Compartir');
  assert.ok(share, 'the real Share action must be visible');
  assert.deepEqual(app.calls, [['updater_status', {}], ['get_state', {}]], 'rendering a candidate only polls states and never starts an upload');
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
