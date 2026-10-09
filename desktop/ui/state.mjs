import { previewSource } from './bridge.mjs';

const notifications = new Set(['not_requested', 'requesting', 'granted', 'denied', 'unavailable']);
const text = (value, max) => typeof value === 'string' && value.length <= max;
const id = value => text(value, 200) && value.trim().length > 0;
const integer = (value, max) => Number.isSafeInteger(value) && value >= 0 && value <= max;
const fail = () => { throw new Error('Estado de la aplicación no válido. Vuelve a abrir Uploadimg Desktop.'); };

export function readUiState(value) {
  if (!value || value.simulation !== true || value.folderAvailable !== false ||
      !text(value.platform, 30) || !text(value.status, 500) || !notifications.has(value.notificationStatus) ||
      !['nativeAvailable', 'paused', 'clipboardEnabled', 'clipboardAvailable'].every(key => typeof value[key] === 'boolean') ||
      !Array.isArray(value.pending) || value.pending.length > 3 || !Array.isArray(value.results) || value.results.length > 20 ||
      ![value.selectedCandidateId, value.selectedResultId].every(selected => selected === null || id(selected))) fail();
  const pending = value.pending.map(candidate => {
    if (!candidate || !id(candidate.id) || !['synthetic', 'clipboard'].includes(candidate.source) ||
        !integer(candidate.width, 8192) || !candidate.width || !integer(candidate.height, 8192) || !candidate.height ||
        candidate.width * candidate.height > 16_000_000 || !integer(candidate.inputBytes, 10_000_000) || !candidate.inputBytes ||
        !integer(candidate.expiresInSeconds, 600)) fail();
    return { id: candidate.id, source: candidate.source, width: candidate.width, height: candidate.height,
      inputBytes: candidate.inputBytes, expiresInSeconds: candidate.expiresInSeconds, previewDataUrl: previewSource(candidate.previewDataUrl) };
  });
  const results = value.results.map(result => {
    if (!result || !id(result.id) || !text(result.url, 2048) || !result.url || result.fictitious !== true ||
        !integer(result.expiresInSeconds, 86400)) fail();
    return { id: result.id, url: result.url, fictitious: true, expiresInSeconds: result.expiresInSeconds };
  });
  if (new Set(pending.map(item => item.id)).size !== pending.length || new Set(results.map(item => item.id)).size !== results.length) fail();
  return { simulation: true, nativeAvailable: value.nativeAvailable, platform: value.platform, paused: value.paused,
    clipboardEnabled: value.clipboardEnabled, clipboardAvailable: value.clipboardAvailable, folderAvailable: false,
    notificationStatus: value.notificationStatus, status: value.status, pending, results,
    selectedCandidateId: value.selectedCandidateId, selectedResultId: value.selectedResultId };
}
