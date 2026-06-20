import { IcoPlay, IcoStop, IcoRestart, IcoEdit, IcoTrash, RecvIcon } from '../icons'
import type { ApiViewer } from '../types'
import { STATE_LABELS, STATE_COLORS, RECV_LABELS, recvTypeFromViewer } from '../types'

interface Props {
  viewer: ApiViewer
  onStart: () => void
  onStop: () => void
  onRestart: () => void
  onEdit: () => void
  onDelete: () => void
}

export function ViewerCard({ viewer, onStart, onStop, onRestart, onEdit, onDelete }: Props) {
  const recvType = recvTypeFromViewer(viewer)
  const recvLabel = RECV_LABELS[recvType] ?? recvType
  const stateColor = STATE_COLORS[viewer.status] ?? STATE_COLORS.unknown
  const stateLabel = STATE_LABELS[viewer.status] ?? 'Inconnu'
  const isRunning = viewer.status === 'running'
  const isRemote = recvType === 'remote'

  return (
    <article style={{
      border: `1px solid ${isRemote ? 'var(--k-accent)' : 'var(--k-line)'}`,
      borderLeft: `${isRemote ? '3px' : '1px'} solid ${isRemote ? 'var(--k-accent)' : 'var(--k-line)'}`,
      borderRadius: 10,
      background: 'var(--k-surface)', padding: '13px 14px',
      display: 'flex', flexDirection: 'column', gap: 12,
    }}>
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 12 }}>
        <div style={{ minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 9 }}>
            <span style={{ color: 'var(--k-muted)', flex: 'none', display: 'inline-flex' }}>
              <RecvIcon type={recvType} size={16} />
            </span>
            <h3 style={{
              margin: 0, fontSize: 15, fontWeight: 600, color: 'var(--k-text)',
              whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
            }}>
              {viewer.id}
            </h3>
          </div>
          <div style={{
            fontSize: 12, fontWeight: 500, color: 'var(--k-muted)',
            marginTop: 5, fontFeatureSettings: "'tnum' 1",
          }}>
            {viewer.server}:{viewer.port}
          </div>
          <div style={{ marginTop: 9 }}>
            <span style={{
              display: 'inline-flex', alignItems: 'center', gap: 6,
              fontSize: 11, fontWeight: 600, padding: '3px 9px 3px 8px', borderRadius: 6,
              ...(isRemote
                ? { background: 'var(--k-accent)', color: 'var(--k-accent-text)' }
                : { border: '1px solid var(--k-line)', color: 'var(--k-muted)' }
              ),
            }}>
              <RecvIcon type={recvType} size={13} />
              {recvLabel}
            </span>
          </div>
        </div>
        <StateBadge color={stateColor} label={stateLabel} pulse={isRunning} />
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
        <ActionBtn onClick={onStart} title="Lancer" accent>
          <IcoPlay size={12} /> Lancer
        </ActionBtn>
        <ActionBtn onClick={onStop} title="Arrêter">
          <IcoStop size={12} /> Arrêter
        </ActionBtn>
        <IconBtn onClick={onRestart} title="Redémarrer">
          <IcoRestart size={14} />
        </IconBtn>
        <IconBtn onClick={onEdit} title="Éditer" highlight>
          <IcoEdit size={13} />
        </IconBtn>
        <div style={{ flex: 1 }} />
        <IconBtn onClick={onDelete} title="Supprimer" danger>
          <IcoTrash size={14} />
        </IconBtn>
      </div>
    </article>
  )
}

function StateBadge({ color, label, pulse }: { color: string; label: string; pulse: boolean }) {
  return (
    <div style={{
      flex: 'none', display: 'flex', alignItems: 'center', gap: 7,
      padding: '4px 10px', borderRadius: 6, background: 'var(--k-surface-2)',
    }}>
      <span style={{
        width: 8, height: 8, borderRadius: '50%', flex: 'none',
        background: color,
        animation: pulse ? 'kf-pulse 1.8s ease-in-out infinite' : 'none',
      }} />
      <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--k-text)', whiteSpace: 'nowrap' }}>
        {label}
      </span>
    </div>
  )
}

function ActionBtn({ onClick, title, children, accent }: {
  onClick: () => void; title: string; children: React.ReactNode; accent?: boolean
}) {
  return (
    <button onClick={onClick} title={title} style={{
      display: 'inline-flex', alignItems: 'center', gap: 6,
      height: 30, padding: '0 12px', borderRadius: 7,
      border: '1px solid var(--k-line)', background: 'transparent',
      color: accent ? 'var(--k-accent)' : 'var(--k-text)',
      font: "600 12px 'Inter'", cursor: 'pointer',
    }}>
      {children}
    </button>
  )
}

function IconBtn({ onClick, title, children, danger, highlight }: {
  onClick: () => void; title: string; children: React.ReactNode; danger?: boolean; highlight?: boolean
}) {
  return (
    <button onClick={onClick} title={title} style={{
      display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
      width: 30, height: 30, borderRadius: 7,
      border: '1px solid var(--k-line)', background: 'transparent',
      color: danger ? 'var(--k-faint)' : highlight ? 'var(--k-text)' : 'var(--k-text)',
      cursor: 'pointer',
    }}>
      {children}
    </button>
  )
}
