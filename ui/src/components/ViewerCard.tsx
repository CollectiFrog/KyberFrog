import { IcoRestart, IcoEdit, IcoTrash, IcoPlay, IcoStop, IcoArrowDown, RecvIcon } from '../icons'
import type { ApiViewer } from '../types'
import { STATE_LABELS, STATE_COLORS, RECV_LABELS, recvTypeFromViewer } from '../types'
import type { LangStrings } from '../hooks/useLang'

interface Props {
  viewer: ApiViewer
  t: LangStrings
  onStart: () => void
  onStop: () => void
  onRestart: () => void
  onEdit: () => void
  onDelete: () => void
}

export function ViewerCard({ viewer, t, onStart, onStop, onRestart, onEdit, onDelete }: Props) {
  const recvType = recvTypeFromViewer(viewer)
  const recvLabel = RECV_LABELS[recvType] ?? recvType
  const stateColor = STATE_COLORS[viewer.status] ?? STATE_COLORS.unknown
  const stateLabel = STATE_LABELS[viewer.status] ?? 'Inconnu'
  const isRunning = viewer.status === 'running'

  const toggleBg = isRunning ? 'var(--k-run)' : 'var(--k-start)'

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
          <RecvIcon type={recvType} size={20} />
        </span>
        <div style={{ minWidth: 0, flex: 1, paddingTop: 1 }}>
          <h3 style={{
            margin: 0, fontSize: 15, fontWeight: 600, color: 'var(--k-text)',
            whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
          }}>
            {viewer.id}
          </h3>
          <div style={{
            marginTop: 4, fontSize: 12, fontWeight: 500, color: 'var(--k-muted)',
            fontFeatureSettings: "'tnum' 1",
            whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
          }}>
            {recvLabel} · {viewer.server}:{viewer.port}
          </div>
        </div>
        <div style={{ flex: 'none', display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: 6 }}>
          <span style={{
            display: 'inline-flex', alignItems: 'center', gap: 5,
            fontSize: 9, fontWeight: 700, letterSpacing: '0.08em',
            color: 'var(--k-muted)', textTransform: 'uppercase',
          }}>
            <IcoArrowDown size={11} />
            {t.reception}
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
          style={{ background: toggleBg, color: '#fff' }}
        >
          {isRunning ? <IcoStop size={15} /> : <IcoPlay size={15} />}
          {isRunning ? t.stop : t.start}
        </BarBtn>
        <BarBtn onClick={onRestart} borderRight>
          <IcoRestart size={15} />
          {t.restart}
        </BarBtn>
        <BarBtn onClick={onEdit} borderRight>
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

function BarBtn({ onClick, children, borderRight, danger, style }: {
  onClick: () => void
  children: React.ReactNode
  borderRight?: boolean
  danger?: boolean
  style?: React.CSSProperties
}) {
  return (
    <button
      onClick={onClick}
      style={{
        flex: 1, height: 54, border: 'none',
        borderRight: borderRight ? '1px solid var(--k-line)' : undefined,
        background: 'transparent',
        color: danger ? 'var(--k-faint)' : 'var(--k-text)',
        cursor: 'pointer',
        display: 'inline-flex', flexDirection: 'column',
        alignItems: 'center', justifyContent: 'center',
        gap: 3,
        font: "600 9px 'Inter'",
        letterSpacing: '0.04em', textTransform: 'uppercase',
        ...style,
      }}
    >
      {children}
    </button>
  )
}
