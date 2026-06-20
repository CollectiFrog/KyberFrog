import { useState } from 'react'
import { IcoClose, IcoChevronLeft, IcoSpout, IcoScreen, IcoSoon, IcoCheck } from '../icons'
import { useSpoutSenders } from '../hooks/useSpoutSenders'
import { useAddTransmitter } from '../hooks/useStatus'
import type { SourceType } from '../types'
import { SRC_LABELS } from '../types'

interface Props {
  onClose: () => void
}

interface SrcTile {
  key: SourceType
  label: string
  desc: string
  available: boolean
}

const SOURCE_TILES: SrcTile[] = [
  { key: 'spout',  label: SRC_LABELS.spout,  desc: 'Flux partagé (Resolume, MadMapper, etc.)', available: true },
  { key: 'screen', label: SRC_LABELS.screen, desc: 'Diffuser un écran de cette machine', available: true },
  { key: 'ndi',    label: SRC_LABELS.ndi,    desc: 'Protocole à venir', available: false },
  { key: 'srt',    label: SRC_LABELS.srt,    desc: 'Protocole à venir', available: false },
  { key: 'syphon', label: SRC_LABELS.syphon, desc: 'Protocole à venir', available: false },
]

export function AddTransmitterDrawer({ onClose }: Props) {
  const [step, setStep] = useState<1 | 2>(1)
  const [srcType, setSrcType] = useState<SourceType | null>(null)
  const [spoutSource, setSpoutSource] = useState<string | null>(null)
  const [port, setPort] = useState('')

  const { data: senders } = useSpoutSenders(step === 2 && srcType === 'spout')
  const addTx = useAddTransmitter()

  const pickType = (t: SourceType) => { setSrcType(t); setStep(2) }
  const back = () => { setStep(1); setSrcType(null); setSpoutSource(null) }

  const canSubmit = srcType === 'spout' ? !!spoutSource : true
  const submitDisabled = !canSubmit || addTx.isPending

  const submit = () => {
    if (!srcType) return
    const portNum = port && /^\d+$/.test(port) ? parseInt(port, 10) : undefined
    addTx.mutate(
      srcType === 'spout'
        ? { kind: 'spout', sender: spoutSource!, port: portNum }
        : { kind: 'screen', port: portNum },
      { onSuccess: onClose }
    )
  }

  const spoutList = senders?.names ?? []
  const activeSpout = senders?.active ?? null

  return (
    <aside className="kf-drawer" style={drawerStyle}>
      <div style={drawerHeaderStyle}>
        <div>
          <div style={tagStyle}>Émission</div>
          <h2 style={h2Style}>Ajouter un transmetteur</h2>
        </div>
        <button onClick={onClose} style={closeBtn}><IcoClose size={17} /></button>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: 20 }}>
        {step === 1 && (
          <>
            <div style={sectionLabel}>1 · Type de source</div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 9 }}>
              {SOURCE_TILES.map(s => (
                <button
                  key={s.key}
                  onClick={() => s.available && pickType(s.key)}
                  disabled={!s.available}
                  style={tileBtnStyle(s.available, false)}
                >
                  <span style={{ flex: 'none', display: 'inline-flex', color: s.available ? 'var(--k-accent)' : 'var(--k-faint)' }}>
                    {s.key === 'spout' ? <IcoSpout size={18} /> : s.key === 'screen' ? <IcoScreen size={18} /> : <IcoSoon size={18} />}
                  </span>
                  <span style={{ flex: 1, textAlign: 'left' }}>
                    <span style={{ display: 'block', fontSize: 14, fontWeight: 600, color: 'var(--k-text)' }}>{s.label}</span>
                    <span style={{ display: 'block', fontSize: 12, color: 'var(--k-muted)', marginTop: 2 }}>{s.desc}</span>
                  </span>
                  {!s.available && <SoonBadge />}
                </button>
              ))}
            </div>
          </>
        )}

        {step === 2 && srcType && (
          <>
            <button onClick={back} style={{ display: 'inline-flex', alignItems: 'center', gap: 6, background: 'none', border: 'none', color: 'var(--k-accent)', font: "600 13px 'Inter'", cursor: 'pointer', padding: 0, marginBottom: 16 }}>
              <IcoChevronLeft size={15} /> Changer de type
            </button>

            <div style={{ display: 'flex', alignItems: 'center', gap: 9, marginBottom: 18, padding: '11px 13px', border: '1px solid var(--k-line)', borderRadius: 8, background: 'var(--k-surface)' }}>
              <span style={{ color: 'var(--k-accent)', display: 'inline-flex' }}>
                {srcType === 'spout' ? <IcoSpout size={17} /> : <IcoScreen size={17} />}
              </span>
              <span style={{ fontSize: 14, fontWeight: 600, color: 'var(--k-text)' }}>{SRC_LABELS[srcType]}</span>
            </div>

            {srcType === 'spout' && (
              <>
                <div style={sectionLabel}>Sources Spout détectées</div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 7, marginBottom: 20 }}>
                  {spoutList.length === 0 && (
                    <div style={{ fontSize: 13, color: 'var(--k-faint)', padding: '12px 0' }}>Aucune source Spout détectée.</div>
                  )}
                  {[...spoutList].sort((a, b) => (b === activeSpout ? 1 : 0) - (a === activeSpout ? 1 : 0)).map(name => {
                    const isActive = name === activeSpout
                    const selected = name === spoutSource
                    return (
                      <button
                        key={name}
                        onClick={() => setSpoutSource(name)}
                        style={{
                          display: 'flex', alignItems: 'center', gap: 11, width: '100%',
                          textAlign: 'left', padding: '11px 13px', borderRadius: 8, cursor: 'pointer',
                          border: `${selected ? '1.5px' : '1px'} solid ${selected ? 'var(--k-accent)' : 'var(--k-line)'}`,
                          background: selected ? 'var(--k-accent-soft)' : 'var(--k-surface)',
                        }}
                      >
                        <span style={{ width: 8, height: 8, borderRadius: '50%', flex: 'none', background: isActive ? 'var(--k-run)' : 'var(--k-faint)' }} />
                        <span style={{ flex: 1, fontSize: 14, color: 'var(--k-text)' }}>{name}</span>
                        {isActive && <span style={{ fontSize: 11, fontWeight: 600, color: 'var(--k-accent)' }}>active</span>}
                        {selected && <IcoCheck size={16} />}
                      </button>
                    )
                  })}
                </div>
              </>
            )}

            <div>
              <label style={fieldLabel}>Port (optionnel)</label>
              <input
                value={port}
                onChange={e => setPort(e.target.value)}
                placeholder="Auto — attribué automatiquement"
                inputMode="numeric"
                style={inputStyle}
              />
            </div>
          </>
        )}
      </div>

      {step === 2 && (
        <div style={footerStyle}>
          <button onClick={onClose} style={cancelBtn}>Annuler</button>
          <div style={{ flex: 1 }} />
          <button onClick={submit} disabled={submitDisabled} style={submitBtnStyle(!submitDisabled)}>
            {addTx.isPending ? 'Création…' : 'Ajouter le transmetteur'}
          </button>
        </div>
      )}
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
const fieldLabel: React.CSSProperties = { display: 'block', fontSize: 12, fontWeight: 600, color: 'var(--k-muted)', marginBottom: 7 }
const inputStyle: React.CSSProperties = { width: '100%', boxSizing: 'border-box', height: 40, padding: '0 13px', background: 'var(--k-input)', border: '1px solid var(--k-line)', borderRadius: 8, color: 'var(--k-text)', font: "500 14px 'Inter'", outline: 'none' }
const footerStyle: React.CSSProperties = { flex: 'none', display: 'flex', alignItems: 'center', gap: 10, padding: '16px 20px', borderTop: '1px solid var(--k-line)' }
const cancelBtn: React.CSSProperties = { height: 40, padding: '0 16px', borderRadius: 8, border: '1px solid var(--k-line)', background: 'transparent', color: 'var(--k-text)', font: "600 13px 'Inter'", cursor: 'pointer' }
function submitBtnStyle(active: boolean): React.CSSProperties {
  return { height: 40, padding: '0 18px', borderRadius: 8, border: 'none', background: 'var(--k-accent)', color: 'var(--k-accent-text)', font: "600 13px 'Inter'", cursor: active ? 'pointer' : 'not-allowed', opacity: active ? 1 : 0.4 }
}
