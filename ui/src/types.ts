export type KfState = 'running' | 'starting' | 'restarting' | 'stopped' | 'unknown';

export type SourceType = 'spout' | 'screen' | 'camera' | 'decklink' | 'ndi' | 'srt' | 'syphon';

export type RecvType = 'display' | 'spout-relay' | 'remote' | 'ndi-relay' | 'record';

export interface ApiSource {
  type: 'spout' | 'screen' | 'camera' | 'decklink' | 'all';
  sender?: string;
  device?: string;
  /** Camera only: options for the device's demuxer (FFmpeg), e.g. input_format. */
  options?: Record<string, string>;
  /** decklink only: physical connector (sdi/hdmi/optical_sdi/...). */
  video_input?: string | null;
  /** decklink only: forced capture mode (BMD FOURCC, e.g. "Hi60"). */
  format_code?: string | null;
  /** screen only: a made-up screen for a machine with no monitor (#54). */
  virtual_display?: VirtualDisplay | null;
}

/** Size of a virtual screen (#54). */
export interface VirtualDisplay {
  width: number;
  height: number;
  refresh_rate?: number;
}

/** GET /virtual-display: can this machine make up a screen? */
export interface VirtualDisplayAvailability {
  available: boolean;
  reason?: string;
}

/** One DeckLink capture mode a card advertises (GET /decklink-formats). */
export interface DecklinkFormat {
  code: string;
  description: string;
}

export interface ApiTransmitter {
  name: string;
  port: number;
  source: ApiSource;
  status: KfState;
  /** Its hardware encoder failed: it runs on x264 until the encoder setting changes or the app restarts. */
  encoder_fallback?: boolean;
  /** What keeps the source from delivering pictures (read from the log, or the Spout registry). */
  source_issue?: { code: string; detail?: string };
  /** A Spout source as the registry describes it; format_name absent = unreadable format. */
  spout?: { width: number; height: number; format: number; format_name?: string };
}

export interface ApiViewer {
  id: string;
  server: string;
  port: number;
  /** 0-based index into the emitter's display list; absent = default display. */
  display_idx?: number | null;
  /** Local monitor the window goes on (0-based); absent = primary monitor. */
  output_monitor?: number | null;
  fullscreen: boolean;
  spout_out?: string | null;
  remote_control: boolean;
  enabled: boolean;
  status: KfState;
}

/** A monitor of this machine, in the order a viewer's output_monitor indexes (GET /monitors). */
export interface LocalMonitor {
  index: number;
  x: number;
  y: number;
  width: number;
  height: number;
  primary: boolean;
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

/** Machine video encoder setting ('auto' = hardware encoder of the primary GPU, else x264). */
export type EncoderId = 'auto' | 'x264' | 'amf' | 'nvenc' | 'qsv';

export interface EncoderInfo {
  /** The stored setting. */
  choice: EncoderId;
  /** What it resolves to on this machine (written into transmitter configs). */
  resolved: Exclude<EncoderId, 'auto'>;
  /** Primary GPU name, when detected. */
  gpu: string | null;
  /** Every choice, with whether it can work on this GPU. */
  options: { id: EncoderId; available: boolean }[];
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
  /** Machine video encoder setting and the choices this GPU allows. */
  encoder: EncoderInfo;
  /** OS of the *server* ('windows' | 'linux' | ...). Spout and "Tout envoyer"
   *  only exist on Windows, so the source picker hides them elsewhere. */
  platform: string;
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
  /** Local output monitor index, as a string ('' = primary monitor). */
  outputMonitor: string;
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
  capture: 'Boîtier de capture',
  decklink: 'DeckLink',
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
    outputMonitor: v.output_monitor != null ? String(v.output_monitor) : '',
    recvType: recvTypeFromViewer(v),
    fullscreen: v.fullscreen,
  };
}

/**
 * Why `name` cannot name a transmitter or viewer (the server would silently
 * keep the old one), or `null` when it can. `taken` lists the names already in
 * use by others of the same kind; `current` is the edited item's own name.
 */
export function nameError(name: string, taken: string[], current?: string): string | null {
  const n = name.trim()
  if (n === '' || n === current) return null
  if (!/^[A-Za-z0-9-]+$/.test(n)) return 'Lettres sans accent, chiffres et tirets uniquement.'
  if (taken.includes(n)) return 'Ce nom est déjà pris.'
  return null
}
