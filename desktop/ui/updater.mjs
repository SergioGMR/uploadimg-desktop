const states = new Set(['disabled', 'idle', 'checking', 'upToDate', 'available', 'downloading', 'installing', 'failed']);
const busyStates = new Set(['checking', 'downloading', 'installing']);
const bounded = (value, limit) => typeof value === 'string' && value.length <= limit;
// Validate the IPC representation only; version precedence/channel policy belongs to Rust.
const numeric = '(?:0|[1-9][0-9]*)';
const identifier = '(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)';
const versionPattern = new RegExp(`^${numeric}\\.${numeric}\\.${numeric}(?:-${identifier}(?:\\.${identifier})*)?(?:\\+[0-9A-Za-z-]+(?:\\.[0-9A-Za-z-]+)*)?$`);
const validVersion = value => bounded(value, 128) && versionPattern.test(value);
const nullableText = (value, limit) => value === null || bounded(value, limit);

export function readUpdaterStatus(value) {
  if (!value || !states.has(value.state) || !validVersion(value.currentVersion) ||
      !['beta', 'stable'].includes(value.channel) || !bounded(value.message, 500) ||
      !nullableText(value.recoveryPath, 4096) || !nullableText(value.blockedReason, 500) ||
      typeof value.canInstall !== 'boolean' ||
      !(value.offeredVersion === null || (validVersion(value.offeredVersion) &&
        value.offeredVersion.split('+')[0] !== value.currentVersion.split('+')[0])) ||
      (value.state === 'available' && value.offeredVersion === null) ||
      (value.canInstall && (value.state !== 'available' || value.blockedReason !== null || value.recoveryPath !== null))) {
    throw new Error('Estado de actualización no válido. Vuelve a buscar actualizaciones.');
  }
  return Object.fromEntries(['state', 'currentVersion', 'offeredVersion', 'channel', 'message',
    'recoveryPath', 'canInstall', 'blockedReason'].map(key => [key, value[key]]));
}

const labels = {
  disabled: 'Actualizaciones no disponibles.', idle: 'Puedes buscar actualizaciones.',
  checking: 'Buscando actualizaciones…', upToDate: 'Tu versión está al día.',
  available: 'Hay una actualización disponible.', downloading: 'Descargando la actualización…',
  installing: 'Instalando la actualización. UploadIMG se reiniciará cuando termine.',
  failed: 'La actualización no se pudo completar.',
};
const setText = (node, value) => { if (node.textContent !== value) node.textContent = value; };
const errorText = error => String(error instanceof Error ? error.message : error || 'No se pudo consultar la actualización.').slice(0, 500);

// Separate from the image bridge: a long check/download must not hold its action queue.
// Status polls may report native progress during an action, but never overtake its result.
export function createUpdaterPanel({ elements, invoke } = {}) {
  const available = typeof invoke === 'function';
  let current = null, operation = false, polling = false, disposed = false, revision = 0;
  let consentVersion = null, actionError = '', pollError = '', pollFailures = 0;
  const node = id => elements[`update-${id}`];
  const eligible = () => available && !disposed && !operation && !actionError && !pollError &&
    current?.state === 'available' && current.canInstall;
  const canCheck = () => available && !disposed && !operation && current?.state !== 'disabled' &&
    (Boolean(actionError || pollError) || !busyStates.has(current?.state));

  function render() {
    if (consentVersion && (!eligible() || current.offeredVersion !== consentVersion)) consentVersion = null;
    setText(node('version'), current ? `Versión actual: ${current.currentVersion}` : 'Versión actual: pendiente de consultar');
    setText(node('offer'), current?.offeredVersion ? `Nueva versión: ${current.offeredVersion}` : '');
    node('offer').hidden = !current?.offeredVersion;
    setText(node('status'), !available ? 'Abre UploadIMG para consultar las actualizaciones.' :
      actionError || pollError || (current ? `${labels[current.state]}${current.message ? ` ${current.message}` : ''}` : 'Consultando actualizaciones…'));
    setText(node('blocked'), current?.blockedReason || '');
    node('blocked').hidden = !current?.blockedReason;
    setText(node('recovery'), current?.recoveryPath
      ? `Recuperación manual: conserva la aplicación anterior en ${current.recoveryPath}. Restaura esa copia antes de volver a instalar; no se reiniciará automáticamente.` : '');
    node('recovery').hidden = !current?.recoveryPath;
    setText(node('check'), 'Buscar actualizaciones');
    node('check').disabled = !canCheck() || Boolean(consentVersion);
    setText(node('install'), 'Instalar y reiniciar');
    node('install').hidden = current?.state !== 'available';
    node('install').disabled = !eligible() || Boolean(consentVersion);
    node('consent').hidden = !consentVersion;
    setText(node('consent-text'), consentVersion
      ? `Se instalará ${consentVersion} y se reiniciará UploadIMG. Se descartará el historial de resultados guardado en memoria. Los enlaces ya publicados conservan su caducidad.` : '');
    setText(node('confirm'), 'Confirmar instalación y reinicio');
    node('confirm').disabled = !consentVersion || !eligible();
    setText(node('cancel'), 'Cancelar');
    node('cancel').disabled = !consentVersion || operation || disposed;
  }

  async function run(command, args) {
    operation = true;
    consentVersion = null;
    actionError = ''; pollError = ''; pollFailures = 0;
    ++revision;
    render(); // Synchronous lock before invoking: repeated clicks never queue another operation.
    try {
      const value = readUpdaterStatus(await invoke(command, args));
      if (!disposed) current = value;
    } catch (error) {
      if (!disposed) actionError = errorText(error);
    } finally {
      operation = false;
      ++revision;
      if (!disposed) render();
    }
  }

  function check() {
    if (!canCheck() || consentVersion) return;
    return run('updater_check', {});
  }
  function requestInstall() {
    if (!eligible() || consentVersion) return;
    consentVersion = current.offeredVersion;
    render();
  }
  function confirm() {
    if (!consentVersion || !eligible() || current.offeredVersion !== consentVersion) return;
    return run('updater_install', { version: consentVersion, confirmRestart: true });
  }
  function cancel() { if (!operation && !disposed) { consentVersion = null; render(); } }
  const listeners = [['check', check], ['install', requestInstall], ['confirm', confirm], ['cancel', cancel]];
  for (const [id, handler] of listeners) node(id).addEventListener('click', handler);
  render();

  return {
    async poll() {
      if (!available || disposed || polling || pollFailures >= 3) return false;
      polling = true;
      const startedAt = revision;
      try {
        const value = readUpdaterStatus(await invoke('updater_status', {}));
        if (!disposed && startedAt === revision) {
          current = value; pollError = ''; pollFailures = 0;
          render();
        }
        return true;
      } catch (error) {
        if (!disposed && startedAt === revision) {
          pollError = errorText(error); ++pollFailures;
          render();
        }
        return false;
      } finally { polling = false; }
    },
    dispose() {
      disposed = true; ++revision; consentVersion = null;
      for (const [id, handler] of listeners) node(id).removeEventListener('click', handler);
      render();
    },
  };
}
