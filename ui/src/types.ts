export type KfState = 'running' | 'starting' | 'restarting' | 'stopped' | 'unknown';

export type SourceType = 'spout' | 'screen' | 'ndi' | 'srt' | 'syphon';

export type RecvType = 'display' | 'spout-relay' | 'remote' | 'ndi-relay' | 'record';

export interface ApiSource {
  type: 'spout' | 'screen';
  sender?: string;
  display?: string;
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
  fullscreen: boolean;
  spout_out?: string | null;
  remote_control: boolean;
  enabled: boolean;
  status: KfState;
}

export interface StatusPayload {
  hostname: string;
  ips: string[];
  transmitters: ApiTransmitter[];
  viewers: ApiViewer[];
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
  recvType: RecvType;
  fullscreen: boolean;
}

export interface AddTxFormState {
  step: 1 | 2;
  srcType: SourceType | null;
  spoutSource: string | null;
  screen: string;
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
    recvType: recvTypeFromViewer(v),
    fullscreen: v.fullscreen,
  };
}
