import { useState, useEffect } from 'react'
import { IcoClose, IcoDisplay, IcoSpoutRelay, IcoRemote, IcoNdi, IcoRecord, IcoSoon, IcoCheck, IcoLock, IcoRestart } from '../icons'
import { useCreateViewer, useUpdateViewer } from '../hooks/useStatus'
import { useDisplays } from '../hooks/useDisplays'
import type { ApiViewer, RecvType, ViewerFormState } from '../types'
import { RECV_LABELS, viewerToFormState } from '../types'

interface RecvTile {
  key: RecvType
  label: string
  desc: string
  available: boolean
}

const RECV_TILES: RecvTile[] = [
  { key: 'display',     label: RECV_LABELS.display,     desc: 'Afficher le flux (plein écran possible)', available: true },
  { key: 'spout-relay', label: RECV_LABELS['spout-relay'], desc: 'Re-publier en source Spout locale', available: true },
  { key: 'remote',      label: RECV_LABELS.remote,      desc: 'Piloter la machine — clavier + souris, fenêtré', available: true },
  { key: 'ndi-relay',   label: RECV_LABELS['ndi-relay'], desc: 'Protocole à venir', available: false },
  { key: 'record',      label: RECV_LABELS.record,      desc: 'Protocole à venir', available: false },
]

function recvIcon(key: RecvType | string, size = 18) {
  switch (key) {
    case 'display': return <IcoDisplay size={size} />
    case 'spout-relay': return <IcoSpoutRelay size={size} />
    case 'remote': return <IcoRemote size={size} />
    case 'ndi-relay': return <IcoNdi size={size} />
    case 'record': return <IcoRecord size={size} />
    default: return <IcoSoon size={size} />
  }
}

interface Props {
  viewer?: ApiViewer
  onClose: () => void
}

export function ViewerFormDrawer({ viewer, onClose }: Props) {
  const isEdit = !!viewer
  const [form, setForm] = useState<ViewerFormState>(() =>
    viewer ? viewerToFormState(viewer) : { name: '', ip: '', port: '', displayIdx: '', recvType: 'display', fullscreen: true }
  )

  const portNum = parseInt(form.port, 10) || 0

  // The emitter's screen list is fetched against a *committed* target updated
  // only on IP/Port blur (see `onBlur` below), never on every keystroke — so
  // detection is always live but never spams the network. `useDisplays` gates
  // itself on a non-empty server + valid port.
  const [target, setTarget] = useState(() => ({ server: form.ip.trim(), port: portNum }))
  const displaysQ = useDisplays(target.server, target.port)
  const displays = displaysQ.data ?? []

  const commitTarget = () => setTarget({ server: form.ip.trim(), port: parseInt(form.port, 10) || 0 })

  // Manual refresh: re-commit the target (picks up an uncommitted IP/Port) and
  // force a refetch when the target didn't change (refetch bypasses staleTime).
  const refreshDisplays = () => {
    const next = { server: form.ip.trim(), port: parseInt(form.port, 10) || 0 }
    if (next.server === target.server && next.port === target.port) {
      displaysQ.refetch()
    } else {
      setTarget(next)
    }
  }

  useEffect(() => {
    if (viewer) {
      const fs = viewerToFormState(viewer)
      setForm(fs)
      setTarget({ server: fs.ip.trim(), port: parseInt(fs.port, 10) || 0 })
    }
  }, [viewer])

  // No "auto" row anymore: once the emitter's list arrives, pre-select the
  // first display (same semantics as the old default, index 0).
  useEffect(() => {
    if (displays.length > 0) {
      setForm(f => (f.displayIdx === '' ? { ...f, displayIdx: '0' } : f))
    }
  }, [displays])

  const patch = (p: Partial<ViewerFormState>) => setForm(f => ({ ...f, ...p }))

  const pickRecv = (key: RecvType) => {
    const fs = (key === 'remote' || key === 'spout-relay') ? false : form.fullscreen
    patch({ recvType: key, fullscreen: fs })
  }

  const createViewer = useCreateViewer()
  const updateViewer = useUpdateViewer()

  const noFullscreen = form.recvType === 'remote' || form.recvType === 'spout-relay'
  const fsOn = form.fullscreen && !noFullscreen
  const fsHint = form.recvType === 'remote'
    ? 'Indisponible : le bureau à distance s\'ouvre en fenêtré.'
    : form.recvType === 'spout-relay'
    ? 'Indisponible : la redirection Spout n\'a pas de rendu visuel.'
    : 'Ouvrir le viewer en plein écran.'

  const valid = (isEdit || (form.name.trim())) && form.ip.trim() && form.port.trim()
  const isPending = createViewer.isPending || updateViewer.isPending

  const submit = () => {
    if (!valid || isPending) return
    if (isEdit && viewer) {
      updateViewer.mutate({ id: viewer.id, form }, { onSuccess: onClose })
    } else {
      createViewer.mutate(form, { onSuccess: onClose })
    }
  }

  return (
    <aside className="kf-drawer" style={drawerStyle}>
      <div style={drawerHeaderStyle}>
        <div>
          <div style={tagStyle}>Réception</div>
          <h2 style={h2Style}>{isEdit ? 'Modifier le viewer' : 'Créer un viewer'}</h2>
        </div>
        <button onClick={onClose} style={closeBtn}><IcoClose size={17} /></button>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: 20, display: 'flex', flexDirection: 'column', gap: 18 }}>

        {/* Name field */}
        {isEdit ? (
          <div>
            <label style={fieldLabel}>Nom <span style={{ fontWeight: 400 }}>— non modifiable</span></label>
            <div style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '0 14px', height: 40, background: 'var(--k-surface-2)', border: '1px dashed var(--k-line)', borderRadius: 8, color: 'var(--k-muted)', fontSize: 14 }}>
              <IcoLock size={14} />
              {form.name}
            </div>
          </div>
        ) : (
          <div>
            <label style={fieldLabel}>Nom</label>
            <input
              value={form.name}
              onChange={e => patch({ name: e.target.value })}
              placeholder="ex. Mur LED Façade"
              style={inputStyle}
              autoFocus
            />
            <div style={{ fontSize: 11, color: 'var(--k-faint)', marginTop: 6 }}>
              Le nom est défini à la création et ne pourra plus être modifié.
            </div>
          </div>
        )}

        {/* IP + Port */}
        <div style={{ display: 'flex', gap: 12 }}>
          <div style={{ flex: 1 }}>
            <label style={fieldLabel}>Machine émettrice (IP)</label>
            <input
              value={form.ip}
              onChange={e => patch({ ip: e.target.value })}
              onBlur={commitTarget}
              placeholder="192.168.1.x"
              inputMode="decimal"
              style={inputStyle}
            />
          </div>
          <div style={{ width: 104 }}>
            <label style={fieldLabel}>Port</label>
            <input
              value={form.port}
              onChange={e => patch({ port: e.target.value })}
              onBlur={commitTarget}
              placeholder="9000"
              inputMode="numeric"
              style={inputStyle}
            />
          </div>
        </div>

        {/* Recv type */}
        <div>
          <div style={sectionLabel}>Type de réception</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {RECV_TILES.map(r => {
              const checked = form.recvType === r.key
              return (
                <button
                  key={r.key}
                  onClick={() => r.available && pickRecv(r.key)}
                  disabled={!r.available}
                  style={tileBtnStyle(r.available, checked)}
                >
                  <span style={{ flex: 'none', display: 'inline-flex', color: r.available ? 'var(--k-accent)' : 'var(--k-faint)' }}>
                    {recvIcon(r.key, 18)}
                  </span>
                  <span style={{ flex: 1, textAlign: 'left' }}>
                    <span style={{ display: 'block', fontSize: 14, fontWeight: 600, color: 'var(--k-text)' }}>{r.label}</span>
                    <span style={{ display: 'block', fontSize: 12, color: 'var(--k-muted)', marginTop: 2 }}>{r.desc}</span>
                  </span>
                  {!r.available && <SoonBadge />}
                  {checked && r.available && <span style={{ flex: 'none', color: 'var(--k-accent)', display: 'inline-flex' }}><IcoCheck size={18} /></span>}
                </button>
              )
            })}
          </div>
        </div>

        {/* Source screen */}
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
            <div style={{ ...sectionLabel, marginBottom: 0 }}>Écran source</div>
            <button
              type="button"
              onClick={refreshDisplays}
              title="Actualiser la liste des écrans"
              style={refreshBtnStyle}
            >
              <IcoRestart size={13} style={displaysQ.isFetching ? { animation: 'kf-spin 0.8s linear infinite' } : undefined} />
            </button>
          </div>
          <div style={{ fontSize: 12, color: 'var(--k-muted)', marginBottom: 10 }}>
            Quel écran du transmetteur souhaitez-vous diffuser ?
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 7 }}>
            {displays.map((d, i) => {
              const selected = form.displayIdx === String(i)
              return (
                <button key={d.id} type="button" onClick={() => patch({ displayIdx: String(i) })} style={displayRowStyle(selected)}>
                  <span style={{ flex: 1, textAlign: 'left' }}>
                    <span style={{ display: 'block', fontSize: 14, fontWeight: 600, color: 'var(--k-text)' }}>{d.name || `Écran ${i}`}</span>
                    <span style={{ display: 'block', fontSize: 12, color: 'var(--k-muted)', marginTop: 2 }}>{d.width}×{d.height} — index {i}</span>
                  </span>
                  {selected && <span style={{ color: 'var(--k-accent)', display: 'inline-flex' }}><IcoCheck size={16} /></span>}
                </button>
              )
            })}
          </div>

          {displaysQ.isFetching && (
            <div style={{ fontSize: 12, color: 'var(--k-muted)', marginTop: 8 }}>Détection…</div>
          )}
          {displaysQ.isError && (
            <div style={{ fontSize: 12, color: '#e0955c', marginTop: 8 }}>
              Émetteur injoignable — impossible de lister les écrans.
            </div>
          )}
          {displaysQ.isSuccess && displays.length === 0 && (
            <div style={{ fontSize: 12, color: 'var(--k-faint)', marginTop: 8 }}>Aucun écran détecté sur l'émetteur.</div>
          )}
        </div>

        {/* Fullscreen toggle */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12, padding: '13px 15px', border: '1px solid var(--k-line)', borderRadius: 8, background: 'var(--k-surface)', opacity: noFullscreen ? 0.55 : 1 }}>
          <div>
            <div style={{ fontSize: 14, fontWeight: 600, color: 'var(--k-text)' }}>Plein écran</div>
            <div style={{ fontSize: 12, color: 'var(--k-muted)', marginTop: 2 }}>{fsHint}</div>
          </div>
          <button
            onClick={() => !noFullscreen && patch({ fullscreen: !form.fullscreen })}
            disabled={noFullscreen}
            style={{
              flex: 'none', position: 'relative', width: 44, height: 26, borderRadius: 999,
              border: 'none', cursor: noFullscreen ? 'not-allowed' : 'pointer',
              background: fsOn ? 'var(--k-accent)' : 'var(--k-line-2)',
              transition: 'background .18s ease',
            }}
          >
            <span style={{
              position: 'absolute', top: 3, left: fsOn ? 21 : 3,
              width: 20, height: 20, borderRadius: '50%', background: '#fff',
              transition: 'left .18s ease', boxShadow: '0 1px 2px rgba(0,0,0,0.3)',
            }} />
          </button>
        </div>
      </div>

      <div style={footerStyle}>
        <button onClick={onClose} style={cancelBtn}>Annuler</button>
        <div style={{ flex: 1 }} />
        <button onClick={submit} disabled={!valid || isPending} style={submitBtnStyle(!!valid && !isPending)}>
          {isPending ? 'Enregistrement…' : isEdit ? 'Enregistrer' : 'Créer le viewer'}
        </button>
      </div>
    </aside>
  )
}

function SoonBadge() {
  return (
    <span style={{ flex: 'none', fontSize: 10, fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--k-faint)', border: '1px solid var(--k-line)', borderRadius: 6, padding: '3px 8px' }}>
      À venir
    </span>
  )
}

function tileBtnStyle(available: boolean, selected: boolean): React.CSSProperties {
  return {
    display: 'flex', alignItems: 'center', gap: 13, width: '100%',
    textAlign: 'left', padding: '13px 14px', borderRadius: 8,
    border: `${selected ? '1.5px' : '1px'} solid ${selected ? 'var(--k-accent)' : 'var(--k-line)'}`,
    background: selected ? 'var(--k-accent-soft)' : 'var(--k-surface)',
    cursor: available ? 'pointer' : 'not-allowed',
    opacity: available ? 1 : 0.45,
  }
}

function displayRowStyle(selected: boolean): React.CSSProperties {
  return {
    display: 'flex', alignItems: 'center', gap: 11, width: '100%',
    textAlign: 'left', padding: '11px 13px', borderRadius: 8, cursor: 'pointer',
    border: `${selected ? '1.5px' : '1px'} solid ${selected ? 'var(--k-accent)' : 'var(--k-line)'}`,
    background: selected ? 'var(--k-accent-soft)' : 'var(--k-surface)',
  }
}

const drawerStyle: React.CSSProperties = {
  position: 'fixed', top: 0, right: 0, bottom: 0, width: 'min(460px, 100vw)',
  zIndex: 90, background: 'var(--k-bg)', borderLeft: '1px solid var(--k-line)',
  boxShadow: '-24px 0 60px rgba(8,11,16,0.4)',
  display: 'flex', flexDirection: 'column', color: 'var(--k-text)',
}
const drawerHeaderStyle: React.CSSProperties = {
  flex: 'none', display: 'flex', alignItems: 'center', justifyContent: 'space-between',
  padding: '18px 20px', borderBottom: '1px solid var(--k-line)',
}
const tagStyle: React.CSSProperties = { fontSize: 11, fontWeight: 700, letterSpacing: '0.12em', textTransform: 'uppercase', color: 'var(--k-accent)' }
const h2Style: React.CSSProperties = { margin: '5px 0 0', fontSize: 20, fontWeight: 700, letterSpacing: '-0.02em', color: 'var(--k-text)', lineHeight: 1 }
const closeBtn: React.CSSProperties = { display: 'inline-flex', alignItems: 'center', justifyContent: 'center', width: 34, height: 34, borderRadius: 8, border: '1px solid var(--k-line)', background: 'transparent', color: 'var(--k-text)', cursor: 'pointer' }
const sectionLabel: React.CSSProperties = { fontSize: 11, fontWeight: 700, letterSpacing: '0.12em', textTransform: 'uppercase', color: 'var(--k-muted)', marginBottom: 10 }
const refreshBtnStyle: React.CSSProperties = { display: 'inline-flex', alignItems: 'center', justifyContent: 'center', width: 22, height: 22, padding: 0, borderRadius: 6, border: '1px solid var(--k-line)', background: 'transparent', color: 'var(--k-muted)', cursor: 'pointer' }
const fieldLabel: React.CSSProperties = { display: 'block', fontSize: 12, fontWeight: 600, color: 'var(--k-muted)', marginBottom: 7 }
const inputStyle: React.CSSProperties = { width: '100%', boxSizing: 'border-box', height: 40, padding: '0 13px', background: 'var(--k-input)', border: '1px solid var(--k-line)', borderRadius: 8, color: 'var(--k-text)', font: "500 14px 'Inter'", outline: 'none' }
const footerStyle: React.CSSProperties = { flex: 'none', display: 'flex', alignItems: 'center', gap: 10, padding: '16px 20px', borderTop: '1px solid var(--k-line)' }
const cancelBtn: React.CSSProperties = { height: 40, padding: '0 16px', borderRadius: 8, border: '1px solid var(--k-line)', background: 'transparent', color: 'var(--k-text)', font: "600 13px 'Inter'", cursor: 'pointer' }
function submitBtnStyle(active: boolean): React.CSSProperties {
  return { height: 40, padding: '0 18px', borderRadius: 8, border: 'none', background: 'var(--k-accent)', color: 'var(--k-accent-text)', font: "600 13px 'Inter'", cursor: active ? 'pointer' : 'not-allowed', opacity: active ? 1 : 0.4 }
}
