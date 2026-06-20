import type { CSSProperties } from 'react'

interface SvgProps {
  size?: number
  style?: CSSProperties
  className?: string
}

const svg = (path: string, fill = false, size = 16, extra?: CSSProperties) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill={fill ? 'currentColor' : 'none'}
    stroke={fill ? 'none' : 'currentColor'}
    strokeWidth={2}
    strokeLinecap="round"
    strokeLinejoin="round"
    style={extra}
    dangerouslySetInnerHTML={{ __html: path }}
  />
)

export const IcoSpout = ({ size = 16 }: SvgProps) => svg(
  '<path d="M5 13a10 10 0 0 1 14 0"/><path d="M8.5 16.5a5 5 0 0 1 7 0"/><path d="M2 9a15 15 0 0 1 20 0"/><circle cx="12" cy="20" r="1"/>',
  false, size
)

export const IcoScreen = ({ size = 16 }: SvgProps) => svg(
  '<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/>',
  false, size
)

export const IcoDisplay = ({ size = 16 }: SvgProps) => svg(
  '<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/>',
  false, size
)

export const IcoSpoutRelay = ({ size = 16 }: SvgProps) => svg(
  '<path d="m17 2 4 4-4 4"/><path d="M3 11v-1a4 4 0 0 1 4-4h14"/><path d="m7 22-4-4 4-4"/><path d="M21 13v1a4 4 0 0 1-4 4H3"/>',
  false, size
)

export const IcoRemote = ({ size = 16 }: SvgProps) => svg(
  '<rect x="2" y="3" width="20" height="13" rx="2"/><path d="M8 21h8M12 16v5"/><path d="m10 7 4 3-4 3z"/>',
  false, size
)

export const IcoNdi = ({ size = 16 }: SvgProps) => svg(
  '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M8 16V8l8 8V8"/>',
  false, size
)

export const IcoRecord = ({ size = 16 }: SvgProps) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="12" r="9" />
    <circle cx="12" cy="12" r="3.5" fill="currentColor" stroke="none" />
  </svg>
)

export const IcoSoon = ({ size = 16 }: SvgProps) => svg(
  '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
  false, size
)

export const IcoPlus = ({ size = 16 }: SvgProps) => svg(
  '<path d="M12 5v14M5 12h14"/>',
  false, size
)

export const IcoRestart = ({ size = 16 }: SvgProps) => svg(
  '<path d="M3 12a9 9 0 1 0 3-6.7"/><path d="M3 4v5h5"/>',
  false, size
)

export const IcoPlay = ({ size = 12 }: SvgProps) => svg('<path d="M8 5v14l11-7z"/>', true, size)

export const IcoStop = ({ size = 12 }: SvgProps) => svg('<rect x="6" y="6" width="12" height="12" rx="1.5"/>', true, size)

export const IcoTrash = ({ size = 14 }: SvgProps) => svg(
  '<path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/>',
  false, size
)

export const IcoEdit = ({ size = 13 }: SvgProps) => svg(
  '<path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z"/>',
  false, size
)

export const IcoClose = ({ size = 17 }: SvgProps) => svg('<path d="M18 6 6 18M6 6l12 12"/>', false, size)

export const IcoInfo = ({ size = 16 }: SvgProps) => svg('<circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/>', false, size)

export const IcoChevronDown = ({ size = 18 }: SvgProps) => svg('<path d="m6 9 6 6 6-6"/>', false, size)

export const IcoChevronLeft = ({ size = 15 }: SvgProps) => svg('<path d="m15 18-6-6 6-6"/>', false, size)

export const IcoCheck = ({ size = 18 }: SvgProps) => svg('<path d="M20 6 9 17l-5-5"/>', false, size)

export const IcoLock = ({ size = 14 }: SvgProps) => svg('<rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>', false, size)

export const IcoFile = ({ size = 14 }: SvgProps) => svg('<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/>', false, size)

export const IcoNetwork = ({ size = 15 }: SvgProps) => svg(
  '<rect x="2" y="14" width="20" height="8" rx="2"/><path d="M6 18h.01M10 18h.01"/><path d="M6 14V8a6 6 0 0 1 12 0v6"/>',
  false, size
)

export const IcoFullscreen = ({ size = 14 }: SvgProps) => svg(
  '<path d="M8 3H5a2 2 0 0 0-2 2v3M21 8V5a2 2 0 0 0-2-2h-3M3 16v3a2 2 0 0 0 2 2h3M16 21h3a2 2 0 0 0 2-2v-3"/>',
  false, size
)

export const IcoSun = ({ size = 18 }: SvgProps) => svg(
  '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
  false, size
)

export const IcoMoon = ({ size = 18 }: SvgProps) => svg('<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9z"/>', false, size)

export const IcoPause = ({ size = 12 }: SvgProps) => svg('<rect x="6" y="4" width="4" height="16" rx="1"/><rect x="14" y="4" width="4" height="16" rx="1"/>', true, size)

import type { SourceType, RecvType } from './types'

export function SourceIcon({ type, size = 16 }: { type: SourceType | string; size?: number }) {
  switch (type) {
    case 'spout': return <IcoSpout size={size} />
    case 'screen': return <IcoScreen size={size} />
    case 'ndi': return <IcoNdi size={size} />
    default: return <IcoSoon size={size} />
  }
}

export function RecvIcon({ type, size = 16 }: { type: RecvType | string; size?: number }) {
  switch (type) {
    case 'display': return <IcoDisplay size={size} />
    case 'spout-relay': return <IcoSpoutRelay size={size} />
    case 'remote': return <IcoRemote size={size} />
    case 'ndi-relay': return <IcoNdi size={size} />
    case 'record': return <IcoRecord size={size} />
    default: return <IcoSoon size={size} />
  }
}
