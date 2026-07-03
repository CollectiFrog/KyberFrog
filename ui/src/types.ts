export type KfState = 'running' | 'starting' | 'restarting' | 'stopped' | 'unknown';

export type SourceType = 'spout' | 'screen' | 'camera' | 'ndi' | 'srt' | 'syphon';

export type RecvType = 'display' | 'spout-relay' | 'remote' | 'ndi-relay' | 'record';

export interface ApiSource {
  type: 'spout' | 'screen' | 'camera' | 'all';
  sender?: string;
  device?: string;
}

export interface ApiTransmitter {
  name: string;
  port: number;
  source: ApiSource;
  status: KfState;
}

export interface ApiViewer {
  id: string;
  server: string;
  port: number;
  /** 0-based index into the emitter's display list; absent = default display. */
  display_idx?: number | null;
  fullscreen: boolean;
  spout_out?: string | null;
  remote_control: boolean;
  enabled: boolean;
  status: KfState;
}

/** One physical display of a remote emitter (GET /displays). */
export interface DisplayInfo {
  id: number;
  name: string;
  width: number;
  height: number;
}

/** One emitter heard on the LAN via mDNS (GET /discovered). */
export interface DiscoveredInstance {
  /** Transmitter name on the announcing machine. */
  name: string;
  /** Announcing machine's hostname (no .local suffix). */
  host: string;
  /** Addresses to reach it, IPv4 first (use the first one). */
  addrs: string[];
  port: number;
  version?: string;
  kind?: string;
  /** The announcer is this very machine. */
  is_self: boolean;
}

export interface UiPrefs {
  theme: 'dark' | 'light';
  lang: 'fr' | 'en';
}

export interface StatusPayload {
  hostname: string;
  ips: string[];
  version: string;
  /** Name of the loaded setup document. */
  active_setup: string;
  /** Every setup available to load. */
  setups: string[];
  /** Machine-side UI preferences. */
  ui: UiPrefs;
  /** "Tout envoyer" mode: one transmitter exposes every source, adds disabled. */
  send_all: boolean;
  transmitters: ApiTransmitter[];
  viewers: ApiViewer[];
}

export interface SetupsView {
  active: string;
  names: string[];
}

export interface SpoutSendersPayload {
  names: string[];
  active: string | null;
}

export interface LogEntry {
  id: string;
  ts: string;
  level: 'INFO' | 'WARN' | 'ERROR' | 'DEBUG';
  src: string;
  msg: string;
}

export type LogSourceId = 'app' | string;

export interface ConfirmState {
  kind: 'tx' | 'viewer';
  id: string;
  name: string;
}

export interface ViewerFormState {
  name: string;
  ip: string;
  port: string;
  /** Selected source-display index, as a string ('' = default display). */
  displayIdx: string;
  recvType: RecvType;
  fullscreen: boolean;
}

export interface AddTxFormState {
  step: 1 | 2;
  srcType: SourceType | null;
  spoutSource: string | null;
  port: string;
}

export const STATE_LABELS: Record<KfState, string> = {
  running: "En cours d'exécution",
  starting: 'Démarrage…',
  restarting: 'Redémarrage…',
  stopped: 'Arrêté',
  unknown: 'Inconnu',
};

export const STATE_COLORS: Record<KfState, string> = {
  running: 'var(--k-run)',
  starting: 'var(--k-start)',
  restarting: 'var(--k-restart)',
  stopped: 'var(--k-muted)',
  unknown: 'var(--k-faint)',
};

export const SRC_LABELS: Record<string, string> = {
  spout: 'Spout',
  screen: "Capture d'écran",
  camera: 'Webcam',
  all: 'Toutes les sources',
  ndi: 'NDI',
  srt: 'SRT',
  syphon: 'Syphon',
};

export const RECV_LABELS: Record<RecvType, string> = {
  display: 'Affichage',
  'spout-relay': 'Redirection Spout',
  remote: 'Bureau à distance',
  'ndi-relay': 'Redirection NDI',
  record: 'Enregistrement',
};

export function recvTypeFromViewer(v: ApiViewer): RecvType {
  if (v.remote_control) return 'remote';
  if (v.spout_out) return 'spout-relay';
  return 'display';
}

export function viewerToFormState(v: ApiViewer): ViewerFormState {
  return {
    name: v.id,
    ip: v.server,
    port: String(v.port),
    displayIdx: v.display_idx != null ? String(v.display_idx) : '',
    recvType: recvTypeFromViewer(v),
    fullscreen: v.fullscreen,
  };
}
