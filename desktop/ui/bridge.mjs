const noArguments = new Set(['get_state', 'generate_synthetic', 'paste_image', 'request_notifications']);

export function commandArguments(command, args = {}) {
  if (!args || typeof args !== 'object' || Array.isArray(args)) throw new Error('Argumentos no válidos.');
  let expected;
  if (noArguments.has(command)) expected = [];
  else if (command === 'set_clipboard_enabled') expected = ['enabled'];
  else if (command === 'set_paused') expected = ['paused'];
  else if (command === 'candidate_action' || command === 'result_action') expected = ['id', 'action'];
  else throw new Error('Comando no permitido.');
  if (Object.keys(args).length !== expected.length || expected.some(key => !Object.hasOwn(args, key))) {
    throw new Error('Argumentos no válidos.');
  }
  if (expected.length === 1 && typeof args[expected[0]] !== 'boolean') throw new Error('Se requiere un valor booleano.');
  if (expected.includes('id')) {
    const actions = command === 'candidate_action' ? ['preview', 'share', 'ignore'] : ['copy', 'open'];
    const opaqueId = command === 'candidate_action' ? /^[0-9a-f]{32}$/ : /^[0-9a-f]{48}$/;
    if (typeof args.id !== 'string' || !opaqueId.test(args.id) || !actions.includes(args.action)) {
      throw new Error('Acción no válida.');
    }
  }
  return Object.fromEntries(expected.map(key => [key, args[key]]));
}

// One native invoke at a time. A user action invalidates any earlier poll snapshot.
export function createBridge({ invoke, onState = () => {}, onError = () => {}, onBusy = () => {} } = {}) {
  const available = typeof invoke === 'function';
  let tail = Promise.resolve(), epoch = 0, pending = 0, polling = false, actionFailed = false;
  function enqueue(command, args) {
    const job = tail.then(() => invoke(command, args));
    tail = job.catch(() => {});
    return job;
  }
  function errorMessage(error) {
    return error instanceof Error ? error.message : String(error || 'No se pudo completar la acción.');
  }
  return {
    available,
    async poll() {
      if (!available || polling || pending) return false;
      polling = true;
      const startedAt = epoch;
      try {
        const value = await enqueue('get_state', {});
        if (startedAt === epoch && pending === 0) {
          onState(value);
          if (!actionFailed) onError('');
        }
        return true;
      } catch (error) {
        if (startedAt === epoch && pending === 0 && !actionFailed) onError(errorMessage(error));
        return false;
      } finally { polling = false; }
    },
    async act(command, args = {}) {
      const payload = commandArguments(command, args);
      if (!available) throw new Error('Abre UploadIMG para utilizar estas acciones.');
      const startedAt = ++epoch;
      if (++pending === 1) onBusy(true);
      try {
        const value = await enqueue(command, payload);
        if (startedAt === epoch) { onState(value); actionFailed = false; onError(''); }
        return value;
      } catch (error) {
        if (startedAt === epoch) { actionFailed = true; onError(errorMessage(error)); }
        throw error;
      } finally {
        if (--pending === 0) onBusy(false);
      }
    },
  };
}

// Native prepares thumbnails; never accept remote URLs, SVG, or full-size images.
const maxPreviewBytes = 1_100_000;
const previewPrefix = 'data:image/png;base64,';
const maxPreviewCharacters = previewPrefix.length + 4 * Math.ceil(maxPreviewBytes / 3);

export function previewSource(value) {
  if (typeof value !== 'string' || value.length > maxPreviewCharacters || !/^data:image\/png;base64,[A-Za-z0-9+/]+={0,2}$/.test(value)) return '';
  try {
    const encoded = value.slice(previewPrefix.length);
    if (encoded.length % 4 !== 0) return '';
    const padding = encoded.endsWith('==') ? 2 : encoded.endsWith('=') ? 1 : 0;
    if (encoded.length / 4 * 3 - padding > maxPreviewBytes) return '';
    // Decode only the PNG header; byte length follows base64 size and padding.
    const header = Uint8Array.from(atob(encoded.slice(0, 44)), char => char.charCodeAt(0));
    const signature = [137, 80, 78, 71, 13, 10, 26, 10];
    if (header.length < 24 || signature.some((byte, index) => header[index] !== byte)) return '';
    const data = new DataView(header.buffer);
    if (data.getUint32(8) !== 13 || data.getUint32(12) !== 0x49484452) return '';
    const width = data.getUint32(16), height = data.getUint32(20);
    return width > 0 && height > 0 && width <= 512 && height <= 512 ? value : '';
  } catch { return ''; }
}
