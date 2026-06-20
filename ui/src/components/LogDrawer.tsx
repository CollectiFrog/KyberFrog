import { useRef, useEffect, useState } from 'react'
import { IcoChevronDown, IcoFile, IcoPause, IcoTrash, IcoFullscreen } from '../icons'
import { api } from '../api'
import type { ApiTransmitter, ApiViewer, LogEntry } from '../types'

interface LogSourceOption {
  value: string
  label: string
  kind: 'app' | 'tx' | 'viewer'
  name: string
}

interface Props {
  transmitters: ApiTransmitter[]
  viewers: ApiViewer[]
  onFullscreen: () => void
}

const LEVEL_COLORS: Record<string, string> = {
  INFO: 'var(--k-accent)',
  WARN: 'var(--k-restart)',
  ERROR: 'var(--k-danger)',
  DEBUG: 'var(--k-faint)',
}

function parseLines(raw: string): LogEntry[] {
  const lines = raw.split('\n')
  const out: LogEntry[] = []
  lines.forEach((line, i) => {
    if (!line.trim()) return
    const tsM = line.match(/(\d{2}:\d{2}:\d{2})/)
    const lvM = line.match(/\[?(INFO|WARN|ERROR|DEBUG)\s*\]?/i)
    const level = (lvM?.[1]?.toUpperCase() ?? 'INFO') as LogEntry['level']
    const ts = tsM?.[1] ?? ''
    const afterLevel = lvM ? line.slice((lvM.index ?? 0) + lvM[0].length).trim() : line.trim()
    out.push({ id: `l${i}`, ts, level, src: '', msg: afterLevel || line.trim() })
  })
  return out
}

export function LogDrawer({ transmitters, viewers, onFullscreen }: Props) {
  const [collapsed, setCollapsed] = useState(false)
  const [source, setSource] = useState('app')
  const [live, setLive] = useState(true)
  const [paused, setPaused] = useState(false)
  const [entries, setEntries] = useState<LogEntry[]>([])
  const lastRef = useRef('')
  const consoleRef = useRef<HTMLDivElement>(null)
  const autoFollow = useRef(true)
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const options: LogSourceOption[] = [
    { value: 'app', label: 'Application (tout)', kind: 'app', name: '' },
    ...transmitters.map(t => ({ value: `tx:${t.name}`, label: `Émission · ${t.name}`, kind: 'tx' as const, name: t.name })),
    ...viewers.map(v => ({ value: `vw:${v.id}`, label: `Réception · ${v.id}`, kind: 'viewer' as const, name: v.id })),
  ]

  const currentOpt = options.find(o => o.value === source) ?? options[0]!

  const fetchLogs = async () => {
    if (paused) return
    try {
      let raw = ''
      if (currentOpt.kind === 'app') raw = await api.logsApp()
      else if (currentOpt.kind === 'tx') raw = await api.logsTransmitter(currentOpt.name)
      else raw = await api.logsViewer(currentOpt.name)
      if (raw === lastRef.current) return
      lastRef.current = raw
      setEntries(parseLines(raw))
    } catch { /* backend may not be up */ }
  }

  useEffect(() => {
    setEntries([])
    lastRef.current = ''
  }, [source])

  useEffect(() => {
    if (!live) { if (timerRef.current) clearInterval(timerRef.current); return }
    void fetchLogs()
    timerRef.current = setInterval(fetchLogs, 2000)
    return () => { if (timerRef.current) clearInterval(timerRef.current) }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [live, source, paused])

  useEffect(() => {
    const el = consoleRef.current
    if (el && autoFollow.current) el.scrollTop = el.scrollHeight
  }, [entries])

  const onScroll = () => {
    const el = consoleRef.current
    if (el) autoFollow.current = (el.scrollHeight - el.scrollTop - el.clientHeight) < 40
  }

  const openFile = () => {
    let f = 'kyberfrog.log'
    if (currentOpt.kind === 'tx') f = `${currentOpt.name}-kycontroller.log`
    else if (currentOpt.kind === 'viewer') f = `kyclient-${currentOpt.name}.log`
    fetch(`/logs/open?file=${encodeURIComponent(f)}`, { method: 'POST' }).catch(() => undefined)
  }

  const logsH = collapsed ? 46 : 'clamp(190px, 26vh, 280px)'

  return (
    <section style={{
      flex: 'none', display: 'flex', flexDirection: 'column',
      height: logsH, overflow: 'hidden', background: 'var(--k-panel)',
    }}>
      {/* toolbar */}
      <div style={{
        flex: 'none', display: 'flex', alignItems: 'center', gap: 10,
        padding: '0 14px', height: 46,
        borderTop: '1px solid var(--k-line)', background: 'var(--k-bar)',
      }}>
        <button
          onClick={() => setCollapsed(c => !c)}
          style={{ display: 'inline-flex', alignItems: 'center', gap: 8, height: 30, padding: '0 8px 0 4px', borderRadius: 7, border: 'none', background: 'transparent', color: 'var(--k-text)', font: "600 13px 'Inter'", cursor: 'pointer' }}
        >
          <span style={{ color: 'var(--k-muted)', display: 'inline-flex', transition: 'transform .2s ease', transform: collapsed ? 'rotate(180deg)' : 'rotate(0deg)' }}>
            <IcoChevronDown size={18} />
          </span>
          <IcoFile size={15} />
          Journaux
        </button>

        <div style={{ width: 1, height: 22, background: 'var(--k-line)' }} />

        <label style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12, fontWeight: 500, color: 'var(--k-muted)' }}>
          Source
          <select
            value={source}
            onChange={e => setSource(e.target.value)}
            style={{ height: 30, padding: '0 10px', background: 'var(--k-input)', border: '1px solid var(--k-line)', borderRadius: 7, color: 'var(--k-text)', font: "500 12px 'Inter'", cursor: 'pointer', maxWidth: 240, outline: 'none' }}
          >
            {options.map(o => <option key={o.value} value={o.value}>{o.label}</option>)}
          </select>
        </label>

        <div style={{ flex: 1 }} />

        <button
          onClick={() => { setLive(l => !l); autoFollow.current = true }}
          style={{ display: 'inline-flex', alignItems: 'center', gap: 7, height: 30, padding: '0 12px', borderRadius: 7, border: `1px solid ${live ? 'var(--k-line-2)' : 'var(--k-line)'}`, background: 'transparent', color: live ? 'var(--k-text)' : 'var(--k-muted)', font: "600 12px 'Inter'", cursor: 'pointer' }}
        >
          <span style={{ width: 7, height: 7, borderRadius: '50%', background: live ? 'var(--k-run)' : 'var(--k-faint)', animation: live ? 'kf-pulse 1.6s ease-in-out infinite' : 'none' }} />
          {live ? 'Live' : 'Live off'}
        </button>

        <button
          onClick={() => { setPaused(p => !p); if (paused) autoFollow.current = false }}
          style={{ display: 'inline-flex', alignItems: 'center', gap: 6, height: 30, padding: '0 12px', borderRadius: 7, border: `1px solid ${paused ? 'var(--k-accent)' : 'var(--k-line)'}`, background: paused ? 'var(--k-accent-soft)' : 'transparent', color: paused ? 'var(--k-accent)' : 'var(--k-text)', font: "600 12px 'Inter'", cursor: 'pointer' }}
        >
          <IcoPause size={12} />
          {paused ? 'Reprendre' : 'Pause'}
        </button>

        <button
          onClick={openFile}
          title="Ouvrir le fichier de log"
          style={tbBtn}
        >
          <IcoFile size={14} /> Fichier
        </button>

        <button
          onClick={() => setEntries([])}
          title="Vider la console"
          style={{ ...tbBtn, color: 'var(--k-muted)' }}
        >
          <IcoTrash size={14} /> Vider
        </button>

        <button onClick={onFullscreen} title="Plein écran" style={{ ...iconBtn }}>
          <IcoFullscreen size={14} />
        </button>
      </div>

      {/* console */}
      {!collapsed && (
        <div
          ref={consoleRef}
          onScroll={onScroll}
          style={{
            flex: 1, minHeight: 0, overflowY: 'auto', padding: '12px 16px',
            fontSize: 12.5, fontFeatureSettings: "'tnum' 1",
            background: 'var(--k-bg)', fontFamily: "'JetBrains Mono', 'Consolas', monospace",
          }}
        >
          {entries.length === 0 && (
            <div style={{ padding: 20, textAlign: 'center', color: 'var(--k-faint)', fontSize: 13 }}>
              Console vide.
            </div>
          )}
          {entries.map(l => (
            <div key={l.id} style={{ display: 'flex', gap: 14, padding: '2px 0', lineHeight: 1.55 }}>
              <span style={{ flex: 'none', width: 64, color: 'var(--k-faint)' }}>{l.ts}</span>
              <span style={{ flex: 'none', width: 50, fontWeight: 700, color: LEVEL_COLORS[l.level] ?? 'var(--k-text)' }}>{l.level}</span>
              <span style={{ opacity: 0.88, color: 'var(--k-text)', wordBreak: 'break-all' }}>{l.msg}</span>
            </div>
          ))}
        </div>
      )}
    </section>
  )
}

const tbBtn: React.CSSProperties = {
  display: 'inline-flex', alignItems: 'center', gap: 6,
  height: 30, padding: '0 12px', borderRadius: 7,
  border: '1px solid var(--k-line)', background: 'transparent',
  color: 'var(--k-text)', font: "600 12px 'Inter'", cursor: 'pointer',
}
const iconBtn: React.CSSProperties = {
  display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
  width: 30, height: 30, borderRadius: 7,
  border: '1px solid var(--k-line)', background: 'transparent',
  color: 'var(--k-text)', cursor: 'pointer',
}
