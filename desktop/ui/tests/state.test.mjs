import test from 'node:test';
import assert from 'node:assert/strict';
import { readUiState } from '../state.mjs';

const fixture = () => ({
  simulation: true, nativeAvailable: true, platform: 'macos', paused: false,
  clipboardEnabled: false, clipboardAvailable: true, folderAvailable: false,
  notificationStatus: 'not_requested', status: 'Listo', pending: [], results: [],
  selectedCandidateId: null, selectedResultId: null,
});

test('accepts the simulated native contract and ignores additive fields', () => {
  const value = fixture();
  assert.deepEqual(readUiState({ ...value, extra: 'ignored' }), value);
});

test('rejects incompatible, unbounded or misleading native summaries', () => {
  for (const change of [
    { simulation: false }, { paused: 'false' }, { folderAvailable: true },
    { notificationStatus: 'perhaps' }, { pending: new Array(4).fill({}) },
    { results: new Array(21).fill({}) }, { status: 'x'.repeat(501) },
    { results: [{ id: 'a', url: 'https://example.invalid/i/a', fictitious: false, expiresInSeconds: 60 }] },
    { pending: [{ id: 'a', source: 'file', width: 1, height: 1, inputBytes: 1, expiresInSeconds: 60, previewDataUrl: '' }] },
  ]) assert.throws(() => readUiState({ ...fixture(), ...change }), /Estado/);
});

test('does not display unsafe thumbnails and keeps URLs as text data', () => {
  const candidate = { id: 'a', source: 'clipboard', width: 20, height: 30, inputBytes: 40, expiresInSeconds: 60, previewDataUrl: 'data:image/svg+xml;base64,PHN2Zz4=' };
  const url = 'https://example.invalid/i/result';
  const value = readUiState({ ...fixture(), pending: [candidate], results: [{ id: 'result', url, fictitious: true, expiresInSeconds: 60 }] });
  assert.equal(value.pending[0].previewDataUrl, '');
  assert.equal(value.results[0].url, url);
});
