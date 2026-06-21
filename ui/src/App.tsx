import { useState, useEffect } from 'react'
import { useNavigate, useLocation, Outlet } from 'react-router-dom'
import { TopBar } from './components/TopBar'
import { TransmitterCard } from './components/TransmitterCard'
import { ViewerCard } from './components/ViewerCard'
import { EmptyState } from './components/EmptyState'
import { LogDrawer } from './components/LogDrawer'
import { AddTransmitterDrawer } from './components/AddTransmitterDrawer'
import { ViewerFormDrawer } from './components/ViewerFormDrawer'
import { AboutModal } from './components/AboutModal'
import { ConfirmDialog } from './components/ConfirmDialog'
import { IcoSpout, IcoDisplay } from './icons'
import { useStatus, useStartTransmitter, useStopTransmitter, useRestartTransmitter, useDeleteTransmitter, useStartViewer, useStopViewer, useRestartViewer, useDeleteViewer } from './hooks/useStatus'
import { useTheme } from './hooks/useTheme'
import type { ConfirmState, ApiViewer } from './types'

type Overlay = 'add-tx' | 'add-viewer' | { editViewer: ApiViewer } | 'about' | 'logs-full' | null

export function App() {
  const { theme, toggle: toggleTheme } = useTheme()
  const { data: status, isError } = useStatus()
  const [overlay, setOverlay] = useState<Overlay>(null)
  const [confirm, setConfirm] = useState<ConfirmState | null>(null)
  const [narrow, setNarrow] = useState(false)
  const navigate = useNavigate()
  const location = useLocation()

  // Sync overlay with route
  useEffect(() => {
    const p = location.pathname
    if (p === '/emission/new') setOverlay('add-tx')
    else if (p === '/reception/new') setOverlay('add-viewer')
    else if (p === '/about') setOverlay('about')
    else if (p === '/logs') setOverlay('logs-full')
    else if (p.startsWith('/reception/') && p !== '/reception/new') {
      const id = p.split('/reception/')[1]
      const v = status?.viewers.find(vw => vw.id === id)
      if (v) setOverlay({ editViewer: v })
    } else {
      setOverlay(null)
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [location.pathname, status])

  useEffect(() => {
    const check = () => setNarrow(window.innerWidth < 1100)
    check()
    window.addEventListener('resize', check)
    return () => window.removeEventListener('resize', check)
  }, [])

  // Update document title
  useEffect(() => {
    if (status?.hostname) document.title = `KyberFrog — ${status.hostname}`
  }, [status?.hostname])

  const close = () => navigate('/', { replace: true })

  // Mutations
  const startTx = useStartTransmitter()
  const stopTx = useStopTransmitter()
  const restartTx = useRestartTransmitter()
  const deleteTx = useDeleteTransmitter()
  const startVw = useStartViewer()
  const stopVw = useStopViewer()
  const restartVw = useRestartViewer()
  const deleteVw = useDeleteViewer()

  const askDelete = (kind: 'tx' | 'viewer', id: string, name: string) =>
    setConfirm({ kind, id, name })

  const doDelete = () => {
    if (!confirm) return
    if (confirm.kind === 'tx') deleteTx.mutate(confirm.id)
    else deleteVw.mutate(confirm.id)
    setConfirm(null)
  }

  const hostname = status?.hostname ?? '…'
  const ip = status?.ips?.[0] ?? '—'
  const version = status?.version ?? '—'
  const online = !isError && !!status

  const showDrawerBg = overlay !== null && overlay !== 'logs-full'
  const showAddTx = overlay === 'add-tx'
  const showAddViewer = overlay === 'add-viewer'
  const showAbout = overlay === 'about'
  const editViewer = typeof overlay === 'object' && overlay !== null && 'editViewer' in overlay ? overlay.editViewer : null

  const mainStyle: React.CSSProperties = {
    flex: 1,
    minHeight: 0,
    display: 'flex',
    flexDirection: narrow ? 'column' : 'row',
    background: 'var(--k-bg)',
  }

  const panelBase: React.CSSProperties = {
    display: 'flex', flexDirection: 'column',
    background: 'var(--k-panel)',
    ...(narrow
      ? { minHeight: '64vh', borderBottom: '1px solid var(--k-line)' }
      : { flex: 1, minHeight: 0 }),
  }

  return (
    <div style={{
      display: 'flex', flexDirection: 'column',
      height: narrow ? 'auto' : '100vh',
      minHeight: '100vh',
      overflow: narrow ? 'auto' : 'hidden',
      background: 'var(--k-bg)', color: 'var(--k-text)',
      fontFamily: "'Inter', system-ui, sans-serif",
    }}>
      <TopBar
        hostname={hostname}
        ip={ip}
        online={online}
        theme={theme}
        onToggleTheme={toggleTheme}
        onAbout={() => navigate('/about')}
      />

      <main style={mainStyle}>
        {/* Émission panel */}
        <section style={{ ...panelBase, ...(narrow ? {} : { borderRight: '1px solid var(--k-line)' }) }}>
          <PaneHeader
            title="Émission"
            count={status?.transmitters.length ?? 0}
            onAdd={() => navigate('/emission/new')}
            addLabel="Ajouter un émetteur"
          />
          <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', padding: 14, display: 'flex', flexDirection: 'column', gap: 10 }}>
            {(!status || status.transmitters.length === 0) && (
              <EmptyState
                icon={<IcoSpout size={32} />}
                title="Aucun transmetteur"
                desc="Ajoutez une source à diffuser sur le réseau local."
              />
            )}
            {status?.transmitters.map(tx => (
              <TransmitterCard
                key={tx.name}
                tx={tx}
                onStart={() => startTx.mutate(tx.name)}
                onStop={() => stopTx.mutate(tx.name)}
                onRestart={() => restartTx.mutate(tx.name)}
                onDelete={() => askDelete('tx', tx.name, tx.name)}
              />
            ))}
          </div>
        </section>

        {/* Réception panel */}
        <section style={panelBase}>
          <PaneHeader
            title="Réception"
            count={status?.viewers.length ?? 0}
            onAdd={() => navigate('/reception/new')}
            addLabel="Ajouter un récepteur"
          />
          <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', padding: 14, display: 'flex', flexDirection: 'column', gap: 10 }}>
            {(!status || status.viewers.length === 0) && (
              <EmptyState
                icon={<IcoDisplay size={32} />}
                title="Aucun viewer"
                desc="Connectez-vous à une machine émettrice du réseau."
              />
            )}
            {status?.viewers.map(v => (
              <ViewerCard
                key={v.id}
                viewer={v}
                onStart={() => startVw.mutate(v.id)}
                onStop={() => stopVw.mutate(v.id)}
                onRestart={() => restartVw.mutate(v.id)}
                onEdit={() => navigate(`/reception/${v.id}`)}
                onDelete={() => askDelete('viewer', v.id, v.id)}
              />
            ))}
          </div>
        </section>
      </main>

      <LogDrawer
        transmitters={status?.transmitters ?? []}
        viewers={status?.viewers ?? []}
        onFullscreen={() => navigate('/logs')}
      />

      {/* Backdrop */}
      {showDrawerBg && (
        <div
          onClick={close}
          style={{ position: 'fixed', inset: 0, background: 'rgba(8,11,16,0.55)', zIndex: 80 }}
        />
      )}

      {/* Overlays */}
      {showAddTx && <AddTransmitterDrawer onClose={close} />}
      {(showAddViewer || editViewer !== null) && (
        <ViewerFormDrawer viewer={editViewer ?? undefined} onClose={close} />
      )}
      {showAbout && (
        <AboutModal
          hostname={hostname}
          ip={ip}
          version={version}
          theme={theme}
          onClose={close}
        />
      )}
      {confirm && (
        <ConfirmDialog
          kind={confirm.kind === 'tx' ? 'transmetteur' : 'viewer'}
          name={confirm.name}
          onConfirm={doDelete}
          onCancel={() => setConfirm(null)}
        />
      )}

      {/* Router outlet for any nested routes */}
      <Outlet />
    </div>
  )
}

function PaneHeader({ title, count, onAdd, addLabel }: { title: string; count: number; onAdd: () => void; addLabel: string }) {
  return (
    <div style={{
      flex: 'none', display: 'flex', alignItems: 'center', justifyContent: 'space-between',
      padding: '14px 18px 13px', borderBottom: '1px solid var(--k-line)',
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700, letterSpacing: '-0.01em', color: 'var(--k-text)', lineHeight: 1 }}>
          {title}
        </h2>
        <span style={{
          display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
          minWidth: 20, height: 20, padding: '0 6px', borderRadius: 6,
          background: 'var(--k-surface-2)', fontSize: 12, fontWeight: 600, color: 'var(--k-muted)',
          fontFeatureSettings: "'tnum' 1",
        }}>
          {count}
        </span>
      </div>
      <button
        onClick={onAdd}
        style={{
          display: 'inline-flex', alignItems: 'center', gap: 7,
          height: 34, padding: '0 14px', borderRadius: 8,
          border: 'none', background: 'var(--k-accent)', color: 'var(--k-accent-text)',
          font: "600 13px 'Inter'", cursor: 'pointer',
        }}
      >
        <svg width={16} height={16} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2.4} strokeLinecap="round">
          <path d="M12 5v14M5 12h14" />
        </svg>
        {addLabel}
      </button>
    </div>
  )
}
