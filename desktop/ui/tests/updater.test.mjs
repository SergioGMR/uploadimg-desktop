import test from 'node:test';
import assert from 'node:assert/strict';
import { createUpdaterPanel, readUpdaterStatus } from '../updater.mjs';

const status = overrides => ({ state: 'idle', currentVersion: '0.1.0-beta.2', offeredVersion: null,
  channel: 'beta', message: '', recoveryPath: null, canInstall: false, blockedReason: null, ...overrides });
const offer = overrides => status({ state: 'available', offeredVersion: '0.1.0-beta.3', canInstall: true, ...overrides });
const deferred = () => { let resolve, reject; const promise = new Promise((yes, no) => { resolve = yes; reject = no; }); return { promise, resolve, reject }; };
const flush = () => new Promise(resolve => setImmediate(resolve));
class Node {
  textContent = ''; hidden = false; disabled = false; listeners = new Map();
  addEventListener(name, callback) { this.listeners.set(name, callback); }
  removeEventListener(name, callback) { if (this.listeners.get(name) === callback) this.listeners.delete(name); }
  click() { if (!this.disabled) return this.listeners.get('click')?.(); }
}
function setup(initial = status(), native = true) {
  const elements = Object.fromEntries(['update-version', 'update-offer', 'update-status', 'update-blocked',
    'update-recovery', 'update-check', 'update-install', 'update-consent', 'update-consent-text',
    'update-confirm', 'update-cancel'].map(id => [id, new Node()]));
  const calls = []; let response = initial;
  const panel = createUpdaterPanel({ elements, invoke: native ? (command, args) => {
    calls.push([command, args]); return typeof response === 'function' ? response(command, args) : Promise.resolve(response);
  } : undefined });
  return { elements, calls, panel, response(value) { response = value; }, async refresh(value) { response = value; await panel.poll(); } };
}

test('IPC status rejects invalid types, inconsistent eligibility and stale or invalid offered versions', () => {
  assert.deepEqual(readUpdaterStatus(offer()), offer());
  for (const value of [null, {}, status({ state: 'unknown' }), status({ canInstall: 'true' }),
    status({ currentVersion: 'latest' }), status({ channel: 'nightly' }), offer({ offeredVersion: null }),
    offer({ offeredVersion: '0.1.0-beta.2' }), offer({ offeredVersion: '0.1.0-01' }),
    offer({ offeredVersion: '<script>' }), offer({ blockedReason: 'Termina la subida' }),
    status({ state: 'upToDate', canInstall: true }), status({ recoveryPath: 3 }), status({ message: 'x'.repeat(501) })]) {
    assert.throws(() => readUpdaterStatus(value), /no válido/);
  }
});

test('native status poll displays full version and does not initiate a check or upload', async () => {
  const app = setup(); await app.panel.poll();
  assert.equal(app.elements['update-version'].textContent, 'Versión actual: 0.1.0-beta.2');
  assert.equal(app.elements['update-check'].textContent, 'Buscar actualizaciones');
  assert.equal(app.elements['update-install'].textContent, 'Instalar y reiniciar');
  assert.equal(app.elements['update-check'].disabled, false);
  assert.equal(app.elements['update-install'].hidden, true);
  assert.deepEqual(app.calls, [['updater_status', {}]]);
});

test('browser and disabled configuration expose availability honestly', async () => {
  const browser = setup(undefined, false); await browser.panel.poll();
  assert.equal(browser.elements['update-check'].disabled, true);
  assert.match(browser.elements['update-status'].textContent, /Abre UploadIMG/);
  assert.equal(browser.calls.length, 0);
  const app = setup(status({ state: 'disabled', message: 'No hay clave de actualización.' })); await app.panel.poll();
  assert.equal(app.elements['update-check'].disabled, true);
  assert.equal(app.elements['update-status'].textContent, 'Actualizaciones no disponibles. No hay clave de actualización.');
  assert.doesNotMatch(app.elements['update-status'].textContent, /al día/);
});

test('offer requires consent, cancel sends nothing, confirmed install uses exact version and restart consent', async () => {
  const app = setup(offer()); await app.panel.poll();
  assert.equal(app.elements['update-offer'].textContent, 'Nueva versión: 0.1.0-beta.3');
  assert.equal(app.elements['update-install'].disabled, false);
  app.elements['update-install'].click();
  assert.equal(app.elements['update-consent'].hidden, false);
  assert.match(app.elements['update-consent-text'].textContent, /reiniciará UploadIMG/);
  assert.match(app.elements['update-consent-text'].textContent, /historial de resultados.*memoria/);
  assert.equal(app.elements['update-cancel'].textContent, 'Cancelar');
  assert.equal(app.calls.length, 1);
  app.elements['update-cancel'].click();
  assert.equal(app.elements['update-consent'].hidden, true);
  assert.equal(app.calls.length, 1);
  app.elements['update-install'].click();
  app.response(status({ state: 'installing', offeredVersion: '0.1.0-beta.3' }));
  await app.elements['update-confirm'].click();
  assert.deepEqual(app.calls.at(-1), ['updater_install', { version: '0.1.0-beta.3', confirmRestart: true }]);
  assert.equal(app.elements['update-consent'].hidden, true);
});

test('backend safety blocks installation while allowing a later explicit check', async () => {
  const app = setup(offer({ canInstall: false, blockedReason: 'Termina o descarta las imágenes pendientes.' })); await app.panel.poll();
  assert.equal(app.elements['update-install'].hidden, false);
  assert.equal(app.elements['update-install'].disabled, true);
  assert.equal(app.elements['update-blocked'].textContent, 'Termina o descarta las imágenes pendientes.');
  assert.equal(app.elements['update-check'].disabled, false);
  app.elements['update-install'].click();
  assert.equal(app.elements['update-consent'].hidden, true);
});

test('changed offer or newly blocked status cancels consent and prevents stale install', async () => {
  for (const changed of [offer({ offeredVersion: '0.1.0-beta.4' }), offer({ canInstall: false, blockedReason: 'Subida en curso.' })]) {
    const app = setup(offer()); await app.panel.poll(); app.elements['update-install'].click();
    await app.refresh(changed); app.elements['update-confirm'].click();
    assert.equal(app.elements['update-consent'].hidden, true);
    assert.equal(app.calls.filter(([command]) => command === 'updater_install').length, 0);
  }
});

test('duplicate clicks are locked synchronously and progress can be polled during a long check', async () => {
  const app = setup(); await app.panel.poll(); const pending = deferred();
  app.response(command => command === 'updater_check' ? pending.promise : Promise.resolve(status({ state: 'checking' })));
  const first = app.elements['update-check'].click(); app.elements['update-check'].click();
  assert.equal(app.elements['update-check'].disabled, true);
  await app.panel.poll();
  assert.equal(app.calls.filter(([command]) => command === 'updater_check').length, 1);
  assert.equal(app.elements['update-status'].textContent, 'Buscando actualizaciones…');
  pending.resolve(status({ state: 'upToDate' })); await first;
  assert.equal(app.elements['update-status'].textContent, 'Tu versión está al día.');
  assert.equal(app.elements['update-check'].disabled, false);
});

test('duplicate confirmation clicks never queue a second install', async () => {
  const app = setup(offer()); await app.panel.poll(); app.elements['update-install'].click(); const pending = deferred();
  app.response(() => pending.promise);
  const first = app.elements['update-confirm'].click(); app.elements['update-confirm'].click();
  assert.equal(app.calls.filter(([command]) => command === 'updater_install').length, 1);
  pending.resolve(status({ state: 'installing', offeredVersion: '0.1.0-beta.3' })); await first;
});

test('an action error after a busy progress status still allows an explicit retry', async () => {
  const app = setup(); await app.panel.poll(); const pending = deferred();
  app.response(command => command === 'updater_check' ? pending.promise : Promise.resolve(status({ state: 'checking' })));
  const check = app.elements['update-check'].click(); await app.panel.poll();
  pending.reject(new Error('Sin conexión.')); await check;
  assert.equal(app.elements['update-check'].disabled, false);
});

test('offline and malformed check responses stay failed through successful polls until explicit retry', async () => {
  for (const result of [() => Promise.reject(new Error('Sin conexión.')), () => Promise.resolve({ state: 'upToDate' })]) {
    const app = setup(); await app.panel.poll(); app.response(result); await app.elements['update-check'].click();
    assert.match(app.elements['update-status'].textContent, /Sin conexión|no válido/);
    await app.refresh(status({ state: 'upToDate' }));
    assert.doesNotMatch(app.elements['update-status'].textContent, /al día/);
    await app.elements['update-check'].click();
    assert.equal(app.elements['update-status'].textContent, 'Tu versión está al día.');
  }
});

test('failed install exposes literal recovery path and does not restart or offer another install', async () => {
  const app = setup(offer()); await app.panel.poll(); app.elements['update-install'].click();
  app.response(status({ state: 'failed', message: '<img src=x onerror=alert(1)>', recoveryPath: '/Applications/UploadIMG.app.backup' }));
  await app.elements['update-confirm'].click();
  assert.equal(app.elements['update-status'].textContent, 'La actualización no se pudo completar. <img src=x onerror=alert(1)>');
  assert.match(app.elements['update-recovery'].textContent, /Recuperación manual.*\/Applications\/UploadIMG.app.backup/);
  assert.equal(app.elements['update-recovery'].hidden, false);
  assert.equal(app.elements['update-install'].hidden, true);
  assert.equal(app.calls.length, 2);
});

test('status polls coalesce, stop after three errors, and explicit check retries', async () => {
  const app = setup(); const pending = deferred(); app.response(() => pending.promise);
  const first = app.panel.poll(); assert.equal(await app.panel.poll(), false); assert.equal(app.calls.length, 1);
  pending.resolve(status()); await first;
  app.response(() => Promise.reject(new Error('Sin conexión.')));
  for (let i = 0; i < 5; i++) await app.panel.poll();
  assert.equal(app.calls.length, 4);
  assert.equal(app.elements['update-check'].disabled, false);
  app.response(status({ state: 'upToDate' })); await app.elements['update-check'].click(); await app.panel.poll();
  assert.equal(app.calls.length, 6);
});

test('a late poll cannot replace a completed manual check and cleanup ignores pending responses', async () => {
  const app = setup(); const pending = deferred();
  app.response(command => command === 'updater_status' ? pending.promise : Promise.resolve(status({ state: 'upToDate' })));
  const poll = app.panel.poll(); await app.elements['update-check'].click(); pending.resolve(status()); await poll;
  assert.equal(app.elements['update-status'].textContent, 'Tu versión está al día.');
  const late = deferred(); app.response(() => late.promise); const request = app.panel.poll();
  app.panel.dispose(); late.resolve(offer()); await request; await flush();
  assert.equal(app.elements['update-status'].textContent, 'Tu versión está al día.');
  assert.equal(app.elements['update-check'].disabled, true);
  assert.equal(await app.panel.poll(), false);
});
