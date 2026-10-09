import { previewSource } from './bridge.mjs';

const notifications = new Set(['not_requested', 'requesting', 'granted', 'denied', 'unavailable']);
const text = (value, max) => typeof value === 'string' && value.length <= max;
const candidateId = value => typeof value === 'string' && /^[0-9a-f]{32}$/.test(value);
const resultId = value => typeof value === 'string' && /^[0-9a-f]{48}$/.test(value);
const integer = (value, max) => Number.isSafeInteger(value) && value >= 0 && value <= max;
const fail = () => { throw new Error('Estado de la aplicación no válido. Vuelve a abrir UploadIMG.'); };

export function readUiState(value) {
  if (!value || value.simulation !== false || value.folderAvailable !== false ||
      !text(value.platform, 30) || !text(value.status, 500) || !notifications.has(value.notificationStatus) ||
      !['nativeAvailable', 'paused', 'clipboardEnabled', 'clipboardAvailable'].every(key => typeof value[key] === 'boolean') ||
      !Array.isArray(value.pending) || value.pending.length > 3 || !Array.isArray(value.results) || value.results.length > 20 ||
      !(value.selectedCandidateId === null || candidateId(value.selectedCandidateId)) ||
      !(value.selectedResultId === null || resultId(value.selectedResultId))) fail();
  const pending = value.pending.map(candidate => {
    if (!candidate || !candidateId(candidate.id) || !['synthetic', 'clipboard'].includes(candidate.source) ||
        !integer(candidate.width, 8192) || !candidate.width || !integer(candidate.height, 8192) || !candidate.height ||
        candidate.width * candidate.height > 16_000_000 || !integer(candidate.inputBytes, 10_000_000) || !candidate.inputBytes ||
        !integer(candidate.expiresInSeconds, 600)) fail();
    return { id: candidate.id, source: candidate.source, width: candidate.width, height: candidate.height,
      inputBytes: candidate.inputBytes, expiresInSeconds: candidate.expiresInSeconds, previewDataUrl: previewSource(candidate.previewDataUrl) };
  });
  const results = value.results.map(result => {
    if (!result || !resultId(result.id) || result.url !== `https://uploadimg.vercel.app/i/${result.id}` || result.fictitious !== false ||
        !integer(result.expiresInSeconds, 86400)) fail();
    return { id: result.id, url: result.url, fictitious: false, expiresInSeconds: result.expiresInSeconds };
  });
  let upload = null;
  if (value.upload !== null) {
    if (!value.upload || !candidateId(value.upload.id) || !['uploading', 'recoverable'].includes(value.upload.status)) fail();
    upload = { id: value.upload.id, status: value.upload.status };
  }
  if (new Set(pending.map(item => item.id)).size !== pending.length || new Set(results.map(item => item.id)).size !== results.length) fail();
  return { simulation: false, nativeAvailable: value.nativeAvailable, platform: value.platform, paused: value.paused,
    clipboardEnabled: value.clipboardEnabled, clipboardAvailable: value.clipboardAvailable, folderAvailable: false,
    notificationStatus: value.notificationStatus, status: value.status, pending, results,
    selectedCandidateId: value.selectedCandidateId, selectedResultId: value.selectedResultId, upload };
}
