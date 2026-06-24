import { IcoRestart, IcoEdit, IcoTrash, IcoPlay, IcoStop, IcoArrowUp, SourceIcon } from '../icons'
import type { ApiTransmitter } from '../types'
import { STATE_LABELS, STATE_COLORS, SRC_LABELS } from '../types'
import type { LangStrings } from '../hooks/useLang'

interface Props {
  tx: ApiTransmitter
  t: LangStrings
  onStart: () => void
  onStop: () => void
  onRestart: () => void
  onEdit?: () => void
  onDelete: () => void
}

export function TransmitterCard({ tx, t, onStart, onStop, onRestart, onEdit, onDelete }: Props) {
  const srcType = tx.source.type
  const srcLabel = SRC_LABELS[srcType] ?? srcType
  const stateColor = STATE_COLORS[tx.status] ?? STATE_COLORS.unknown
  const stateLabel = STATE_LABELS[tx.status] ?? 'Inconnu'
  const isRunning = tx.status === 'running'

  const toggleBg = 'var(--k-accent)'
  const toggleColor = 'var(--k-accent-text)'

  return (
    <article style={{
      flex: 'none',
      border: '1px solid var(--k-line)', borderRadius: 10,
      background: 'var(--k-surface)', overflow: 'hidden',
      display: 'flex', flexDirection: 'column',
    }}>
      {/* Header */}
      <div style={{ padding: '14px 15px', display: 'flex', alignItems: 'flex-start', gap: 12 }}>
        <span style={{
          flex: 'none', display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
          width: 40, height: 40, borderRadius: 10,
          background: 'var(--k-surface-2)', color: 'var(--k-accent)',
        }}>
          <SourceIcon type={srcType} size={20} />
        </span>
        <div style={{ minWidth: 0, flex: 1, paddingTop: 1 }}>
          <h3 style={{
            margin: 0, fontSize: 15, fontWeight: 600, color: 'var(--k-text)',
            whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
          }}>
            {tx.name}
          </h3>
          <div style={{ marginTop: 4, fontSize: 12, fontWeight: 500, color: 'var(--k-muted)', fontFeatureSettings: "'tnum' 1" }}>
            {srcLabel} · port {tx.port}
            {tx.source.type === 'spout' && tx.source.sender && ` · ${tx.source.sender}`}
          </div>
        </div>
        <div style={{ flex: 'none', display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: 6 }}>
          <span style={{
            display: 'inline-flex', alignItems: 'center', gap: 5,
            fontSize: 9, fontWeight: 700, letterSpacing: '0.08em',
            color: 'var(--k-muted)', textTransform: 'uppercase',
          }}>
            <IcoArrowUp size={11} />
            {t.emission}
          </span>
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
            <span style={{
              width: 7, height: 7, borderRadius: '50%',
              background: stateColor, flex: 'none',
              animation: isRunning ? 'kf-pulse 2s ease-in-out infinite' : 'none',
            }} />
            <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--k-text)', whiteSpace: 'nowrap' }}>
              {stateLabel}
            </span>
          </span>
        </div>
      </div>

      {/* Action bar */}
      <div style={{ display: 'flex', borderTop: '1px solid var(--k-line)' }}>
        <BarBtn
          onClick={isRunning ? onStop : onStart}
          borderRight
          style={{ background: toggleBg, color: toggleColor }}
        >
          {isRunning ? <IcoStop size={15} /> : <IcoPlay size={15} />}
          {isRunning ? t.stop : t.start}
        </BarBtn>
        <BarBtn onClick={onRestart} borderRight>
          <IcoRestart size={15} />
          {t.restart}
        </BarBtn>
        <BarBtn onClick={onEdit ?? (() => {})} borderRight disabled={!onEdit}>
          <IcoEdit size={14} />
          {t.edit}
        </BarBtn>
        <BarBtn onClick={onDelete} danger>
          <IcoTrash size={15} />
          {t.del}
        </BarBtn>
      </div>
    </article>
  )
}

function BarBtn({ onClick, children, borderRight, danger, disabled, style }: {
  onClick: () => void
  children: React.ReactNode
  borderRight?: boolean
  danger?: boolean
  disabled?: boolean
  style?: React.CSSProperties
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      style={{
        flex: 1, height: 54, border: 'none',
        borderRight: borderRight ? '1px solid var(--k-line)' : 'none',
        background: 'transparent',
        color: danger ? 'var(--k-faint)' : 'var(--k-text)',
        cursor: disabled ? 'default' : 'pointer',
        display: 'inline-flex', flexDirection: 'column',
        alignItems: 'center', justifyContent: 'center',
        gap: 3,
        font: "600 9px 'Inter'",
        letterSpacing: '0.04em', textTransform: 'uppercase',
        opacity: disabled ? 0.4 : 1,
        ...style,
      }}
    >
      {children}
    </button>
  )
}
