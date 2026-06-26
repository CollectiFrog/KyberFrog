import type { StatusPayload, SpoutSendersPayload, RecvType, ViewerFormState, SetupsView, UiPrefs } from './types'

const BASE = ''

async function json<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(BASE + url, init)
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`)
  return res.json() as Promise<T>
}

export const api = {
  status: (): Promise<StatusPayload> =>
    json('/status'),

  spoutSenders: (): Promise<SpoutSendersPayload> =>
    json('/spout-senders'),

  // Transmitters
  addTransmitter: (body: { kind: 'spout' | 'screen'; sender?: string; port?: number }): Promise<StatusPayload> =>
    json('/transmitters', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }),

  startTransmitter: (name: string): Promise<StatusPayload> =>
    json(`/transmitters/${encodeURIComponent(name)}/start`, { method: 'POST' }),

  stopTransmitter: (name: string): Promise<StatusPayload> =>
    json(`/transmitters/${encodeURIComponent(name)}/stop`, { method: 'POST' }),

  restartTransmitter: (name: string): Promise<StatusPayload> =>
    json(`/transmitters/${encodeURIComponent(name)}/restart`, { method: 'POST' }),

  deleteTransmitter: (name: string): Promise<StatusPayload> =>
    json(`/transmitters/${encodeURIComponent(name)}`, { method: 'DELETE' }),

  // Viewers
  createViewer: (form: ViewerFormState): Promise<StatusPayload> =>
    json('/viewers', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(viewerPayload(null, form)),
    }),

  updateViewer: (id: string, form: ViewerFormState): Promise<StatusPayload> =>
    json(`/viewers/${encodeURIComponent(id)}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(viewerPayload(id, form)),
    }),

  startViewer: (id: string): Promise<StatusPayload> =>
    json(`/viewers/${encodeURIComponent(id)}/start`, { method: 'POST' }),

  stopViewer: (id: string): Promise<StatusPayload> =>
    json(`/viewers/${encodeURIComponent(id)}/stop`, { method: 'POST' }),

  restartViewer: (id: string): Promise<StatusPayload> =>
    json(`/viewers/${encodeURIComponent(id)}/restart`, { method: 'POST' }),

  deleteViewer: (id: string): Promise<StatusPayload> =>
    json(`/viewers/${encodeURIComponent(id)}`, { method: 'DELETE' }),

  // Setups (save / load)
  listSetups: (): Promise<SetupsView> =>
    json('/setups'),

  loadSetup: (name: string): Promise<StatusPayload> =>
    json('/setups/load', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name }) }),

  saveSetupAs: (name: string): Promise<StatusPayload> =>
    json('/setups/save-as', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name }) }),

  importSetup: ({ name, text }: { name: string; text: string }): Promise<StatusPayload> =>
    json(`/setups/import?name=${encodeURIComponent(name)}`, { method: 'POST', headers: { 'Content-Type': 'application/toml' }, body: text }),

  /** URL of the download endpoint (used directly by an anchor / window.open). */
  exportSetupUrl: (name?: string): string =>
    name ? `/setups/export?name=${encodeURIComponent(name)}` : '/setups/export',

  // UI preferences (theme / language), persisted machine-side.
  setPrefs: (body: Partial<UiPrefs>): Promise<StatusPayload> =>
    json('/prefs', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }),

  // Logs (polling — SSE not yet available)
  logsApp: (lines = 200): Promise<string> =>
    fetch(`/logs/app?lines=${lines}`).then(r => r.text()),

  logsTransmitter: (name: string, lines = 200): Promise<string> =>
    fetch(`/logs/transmitter/${encodeURIComponent(name)}?lines=${lines}`).then(r => r.text()),

  logsViewer: (id: string, lines = 200): Promise<string> =>
    fetch(`/logs/viewer/${encodeURIComponent(id)}?lines=${lines}`).then(r => r.text()),
}

function viewerPayload(_currentId: string | null, form: ViewerFormState) {
  const recvType: RecvType = form.recvType
  const remote = recvType === 'remote'
  const spoutOut = recvType === 'spout-relay' ? `KyberFrog-${form.name}` : null
  const fullscreen = remote || recvType === 'spout-relay' ? false : form.fullscreen
  return {
    id: form.name.trim() || undefined,
    server: form.ip.trim(),
    port: parseInt(form.port, 10) || 9000,
    fullscreen,
    spout_out: spoutOut,
    remote_control: remote,
  }
}
