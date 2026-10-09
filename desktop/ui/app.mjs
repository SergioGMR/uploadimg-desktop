import { createBridge } from './bridge.mjs';
import { readUiState } from './state.mjs';

const byId = id => document.getElementById(id);
const text = (element, value) => { if (element.textContent !== value) element.textContent = value; };
const elements = Object.fromEntries(['clipboard', 'clipboard-detail', 'pause', 'notifications', 'notification-detail',
  'status', 'busy', 'error', 'paste', 'generate', 'pending-count', 'pending-empty', 'pending-list', 'capture-hint',
  'selected-preview', 'selected-image', 'results-count', 'results-empty', 'results-list', 'browser-notice',
  'upload-panel', 'upload-status', 'recover-upload'].map(id => [id, byId(id)]));
const candidateNodes = new Map(), resultNodes = new Map();
let confirmedState = null, busy = false;

const nativeInvoke = window.__TAURI__?.core?.invoke;
const bridge = createBridge({
  invoke: typeof nativeInvoke === 'function' ? (command, args) => nativeInvoke(command, args) : undefined,
  onState(value) { confirmedState = readUiState(value); render(); },
  onError(message) {
    text(elements.error, String(message).slice(0, 500));
    elements.error.hidden = !message;
  },
  onBusy(value) { busy = value; elements.busy.hidden = !value; updateControls(); },
});

async function act(command, args) {
  if (command === 'candidate_action' && args.action === 'share') {
    if (busy || !confirmedState?.nativeAvailable) return;
    const upload = confirmedState.upload;
    if (upload ? upload.id !== args.id || upload.status !== 'recoverable'
      : !confirmedState.pending.some(item => item.id === args.id)) return;
  }
  try { await bridge.act(command, args); }
  catch { /* The bridge presents the native error; confirmed state stays unchanged. */ }
}

function updateControls() {
  const unavailable = busy || !confirmedState || !bridge.available;
  for (const button of document.querySelectorAll('[data-command]')) button.disabled = unavailable;
  for (const node of candidateNodes.values()) {
    node.share.disabled = unavailable || !confirmedState?.nativeAvailable || confirmedState?.upload !== null;
  }
  elements['recover-upload'].disabled = unavailable || !confirmedState?.nativeAvailable || confirmedState?.upload?.status !== 'recoverable';
  elements.generate.disabled = unavailable;
  elements.pause.disabled = unavailable;
  elements.clipboard.disabled = unavailable || !confirmedState?.clipboardAvailable || !confirmedState?.nativeAvailable;
  elements.paste.disabled = unavailable || !confirmedState?.clipboardAvailable || !confirmedState?.nativeAvailable || confirmedState?.paused;
  elements.notifications.disabled = unavailable || !confirmedState?.nativeAvailable ||
    ['unavailable', 'requesting', 'granted'].includes(confirmedState?.notificationStatus);
  // Keep switches on the last confirmed value even if native rejects a request.
  elements.clipboard.checked = confirmedState?.clipboardEnabled ?? false;
}

function element(tag, className, content) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (content) node.textContent = content;
  return node;
}

function actionButton(label, command, id, action, className = '') {
  const button = element('button', className, label);
  button.type = 'button';
  button.dataset.command = command;
  button.addEventListener('click', () => act(command, { id, action }));
  return button;
}

function newCandidate(id) {
  const root = element('article', 'candidate');
  const thumbnail = element('div', 'thumbnail');
  const image = element('img');
  image.alt = 'Miniatura de la imagen pendiente';
  const unavailable = element('span', 'detail', 'Vista previa no disponible');
  thumbnail.append(image, unavailable);
  const content = element('div', 'card-content');
  const source = element('span', 'card-source');
  const meta = element('p', 'card-meta');
  const expiry = element('p', 'detail');
  const actions = element('div', 'card-actions');
  const share = actionButton('Compartir', 'candidate_action', id, 'share', 'primary');
  share.setAttribute('aria-describedby', 'share-consent');
  const ignore = actionButton('Ignorar', 'candidate_action', id, 'ignore');
  actions.append(share, ignore);
  const preview = actionButton('Ver imagen', 'candidate_action', id, 'preview', 'preview-button');
  preview.setAttribute('aria-pressed', 'false');
  content.append(source, meta, expiry, actions, preview);
  root.append(thumbnail, content);
  return { root, image, unavailable, source, meta, expiry, preview, share };
}

function newResult(id) {
  const root = element('li', 'result');
  const content = element('div', 'result-text');
  const url = element('p', 'result-url');
  const expiry = element('p', 'detail');
  content.append(url, expiry);
  const actions = element('div', 'button-group');
  actions.append(actionButton('Copiar enlace', 'result_action', id, 'copy', 'quiet'),
    actionButton('Abrir enlace', 'result_action', id, 'open', 'quiet'));
  root.append(content, actions);
  return { root, url, expiry };
}

function lifetime(seconds) {
  if (seconds < 60) return `${seconds} s`;
  if (seconds < 3600) return `${Math.ceil(seconds / 60)} min`;
  return `${Math.ceil(seconds / 3600)} h`;
}

function setImage(image, source) {
  if (source) {
    if (image.getAttribute('src') !== source) image.setAttribute('src', source);
    image.hidden = false;
  } else { image.hidden = true; image.removeAttribute('src'); }
}

// Retain each card and focused button across one-second status/expiry updates.
function reconcile(items, nodes, parent, create, update) {
  const active = new Set(items.map(item => item.id));
  for (const [id, node] of nodes) {
    if (!active.has(id)) { node.root.remove(); nodes.delete(id); }
  }
  items.forEach((item, index) => {
    if (!nodes.has(item.id)) nodes.set(item.id, create(item.id));
    const node = nodes.get(item.id);
    update(node, item);
    if (parent.children[index] !== node.root) parent.insertBefore(node.root, parent.children[index] ?? null);
  });
}

function render() {
  const state = confirmedState;
  text(elements.status, state.status || (state.paused ? 'Detección pausada.' : 'Listo.'));
  text(elements.pause, state.paused ? 'Reanudar' : 'Pausar');
  text(elements['clipboard-detail'], !state.clipboardAvailable || !state.nativeAvailable
    ? 'La integración del portapapeles no está disponible en esta plataforma.'
    : state.paused ? 'Detección pausada. Reanuda para detectar nuevas imágenes.'
    : state.clipboardEnabled ? 'Activado. Se detectan imágenes copiadas; compartir siempre requiere tu acción.'
    : 'Desactivado. Actívalo para detectar imágenes del portapapeles.');
  const notificationLabels = {
    not_requested: 'El permiso se solicita solo al pulsar el botón.',
    requesting: 'Esperando la respuesta del sistema…',
    granted: 'Permiso concedido. El sistema controla cuándo se muestran.',
    denied: 'Permiso denegado. Revisa las imágenes desde esta ventana.',
    unavailable: 'Integración nativa pendiente en esta plataforma.',
  };
  text(elements['notification-detail'], notificationLabels[state.notificationStatus]);
  text(elements.notifications, state.notificationStatus === 'granted' ? 'Permiso concedido' :
    state.notificationStatus === 'requesting' ? 'Solicitando…' : 'Solicitar permiso');
  text(elements['capture-hint'], state.platform === 'macos' && state.clipboardAvailable
    ? 'Activa la detección y usa Ctrl + Cmd + Shift + 4 para copiar una captura, o pega una imagen explícitamente.'
    : 'Genera una imagen de prueba para revisar su vista previa. Compartir la enviará al servicio.');
  const upload = state.upload;
  elements['upload-panel'].hidden = upload === null;
  elements['recover-upload'].hidden = upload?.status !== 'recoverable';
  text(elements['upload-status'], upload?.status === 'recoverable'
    ? 'No se pudo confirmar la subida ni su enlace. Recupera el enlace sin volver a enviar la imagen.'
    : 'Subida en curso. Esperando el enlace; podrás compartir otra imagen cuando termine.');
  text(elements['pending-count'], String(state.pending.length));
  text(elements['results-count'], String(state.results.length));
  elements['pending-empty'].hidden = state.pending.length > 0;
  elements['results-empty'].hidden = state.results.length > 0;
  reconcile(state.pending, candidateNodes, elements['pending-list'], newCandidate, (node, item) => {
    text(node.source, item.source === 'clipboard' ? 'Imagen copiada' : 'Imagen de prueba');
    text(node.meta, `${item.width} × ${item.height} px · ${Math.max(1, Math.round(item.inputBytes / 1000))} kB`);
    text(node.expiry, `Disponible durante ${lifetime(item.expiresInSeconds)}`);
    setImage(node.image, item.previewDataUrl);
    node.unavailable.hidden = Boolean(item.previewDataUrl);
    const selected = state.selectedCandidateId === item.id;
    node.root.classList.toggle('selected', selected);
    node.preview.setAttribute('aria-pressed', String(selected));
  });
  const selected = state.pending.find(item => item.id === state.selectedCandidateId);
  const preview = selected?.previewDataUrl ?? '';
  elements['selected-preview'].hidden = !preview;
  setImage(elements['selected-image'], preview);
  reconcile(state.results, resultNodes, elements['results-list'], newResult, (node, item) => {
    text(node.url, item.url);
    text(node.expiry, `Caduca en ${lifetime(item.expiresInSeconds)}`);
  });
  updateControls();
}

elements['recover-upload'].addEventListener('click', () => {
  const upload = confirmedState?.upload;
  if (upload?.status === 'recoverable') return act('candidate_action', { id: upload.id, action: 'share' });
});

elements.clipboard.addEventListener('change', () => act('set_clipboard_enabled', { enabled: elements.clipboard.checked }));
elements.pause.addEventListener('click', () => act('set_paused', { paused: !confirmedState.paused }));
elements.notifications.addEventListener('click', () => act('request_notifications'));
elements.paste.addEventListener('click', () => act('paste_image'));
elements.generate.addEventListener('click', () => act('generate_synthetic'));

if (!bridge.available) {
  elements['browser-notice'].hidden = false;
  text(elements.status, 'Funciones nativas disponibles dentro de UploadIMG.');
  text(elements['notification-detail'], 'Abre la aplicación para consultar su disponibilidad.');
} else {
  void bridge.poll();
  const interval = window.setInterval(() => { void bridge.poll(); }, 1000);
  window.addEventListener('pagehide', () => window.clearInterval(interval), { once: true });
}
