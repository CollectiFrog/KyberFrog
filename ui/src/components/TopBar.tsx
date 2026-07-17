import { useRef, useState, useCallback } from 'react'
import { IcoGear, IcoNetwork, IcoDownload, IcoUpload, IcoEdit } from '../icons'
import type { LangStrings } from '../hooks/useLang'
import type { Theme } from '../hooks/useTheme'

interface Props {
  hostname: string
  ip: string
  online: boolean
  theme: Theme
  t: LangStrings
  activeSetup: string
  setups: string[]
  exportUrl: string
  onLogoClick: () => void
  onOptions: () => void
  onLoadSetup: (name: string) => void
  onSaveAs: () => void
  onImportFile: (file: File) => void
}

export function TopBar({
  hostname, ip, online, theme, t,
  activeSetup, setups, exportUrl,
  onLogoClick, onOptions, onLoadSetup, onSaveAs, onImportFile,
}: Props) {
  const importRef = useRef<HTMLInputElement>(null)
  const logoClicks = useRef(0)
  const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const [copied, setCopied] = useState(false)
  const copyTimer = useRef<ReturnType<typeof setTimeout> | null>(null)

  const onCopyIp = async () => {
    if (!(await copyText(ip))) return
    setCopied(true)
    if (copyTimer.current) clearTimeout(copyTimer.current)
    copyTimer.current = setTimeout(() => setCopied(false), 1500)
  }

  const handleLogoClick = useCallback(() => {
    if (resetTimer.current) clearTimeout(resetTimer.current)
    logoClicks.current += 1
    if (logoClicks.current >= 5) {
      logoClicks.current = 0
      onLogoClick()
      return
    }
    resetTimer.current = setTimeout(() => { logoClicks.current = 0 }, 2000)
  }, [onLogoClick])

  const onFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (file) onImportFile(file)
    e.target.value = ''
  }

  // The active setup is always offered even if the list hasn't caught up yet.
  const options = setups.includes(activeSetup) ? setups : [activeSetup, ...setups]

  return (
    <header style={{
      flex: 'none',
      height: 56,
      display: 'flex',
      alignItems: 'center',
      gap: 16,
      padding: '0 18px',
      background: 'var(--k-bar)',
      borderBottom: '1px solid var(--k-line)',
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 11 }}>
        <img
          src={theme === 'light' ? '/assets/logo-darkpurple.svg' : '/assets/logo-saffron.svg'}
          alt="KyberFrog"
          width={32}
          height={32}
          style={{ flex: 'none', objectFit: 'contain', cursor: 'pointer', userSelect: 'none' }}
          onClick={handleLogoClick}
          onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
        />
        <span style={{ fontSize: 17, fontWeight: 700, letterSpacing: '-0.02em', color: 'var(--k-text)', lineHeight: 1 }}>
          KyberFrog
        </span>
      </div>

      <div style={{ width: 1, height: 22, background: 'var(--k-line)' }} />

      {/* Identity block: hostname over the IP (click to copy), status LED on the right */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: 3, lineHeight: 1 }}>
          <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--k-text)' }}>
            {hostname}
          </span>
          <button
            onClick={onCopyIp}
            title={copied ? t.copied : t.copyIp}
            style={{
              display: 'inline-flex', alignItems: 'center', gap: 5,
              padding: 0, border: 'none', background: 'transparent',
              color: copied ? 'var(--k-accent)' : 'var(--k-muted)',
              font: "500 12px 'Inter'", fontFeatureSettings: "'tnum' 1",
              cursor: 'pointer', lineHeight: 1,
            }}
          >
            <IcoNetwork size={12} />
            {copied ? t.copied : ip}
          </button>
        </div>
        <span
          title={online ? t.online : t.offline}
          style={{
            flex: 'none', width: 8, height: 8, borderRadius: '50%',
            background: online ? '#3FB85C' : 'var(--k-danger)',
            animation: online ? 'kf-pulse 2s ease-in-out infinite' : 'none',
            cursor: 'help',
          }}
        />
      </div>

      <div style={{ flex: 1 }} />

      {/* Setup: load (picker) / save as / download / import */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <label style={{ display: 'flex', alignItems: 'center', gap: 7, fontSize: 12, fontWeight: 600, color: 'var(--k-muted)' }}>
          {t.setupLabel}
          <select
            value={activeSetup}
            onChange={(e) => onLoadSetup(e.target.value)}
            title={t.loadTitle}
            style={{
              height: 36, padding: '0 10px', maxWidth: 200,
              background: 'var(--k-input)', border: '1px solid var(--k-line)', borderRadius: 8,
              color: 'var(--k-text)', font: "600 13px 'Inter'", cursor: 'pointer', outline: 'none',
            }}
          >
            {options.map(name => <option key={name} value={name}>{name}</option>)}
          </select>
        </label>

        <button onClick={onSaveAs} title={t.saveAsTitle} style={textBtnStyle}>
          <IcoEdit size={15} />
          {t.saveAs}
        </button>

        <a href={exportUrl} download title={t.downloadTitle} style={{ ...iconBtnStyle, textDecoration: 'none' }}>
          <IcoDownload size={16} />
        </a>
        <button onClick={() => importRef.current?.click()} title={t.importTitle} style={iconBtnStyle}>
          <IcoUpload size={16} />
        </button>
        <input ref={importRef} type="file" accept=".toml" onChange={onFileChange} style={{ display: 'none' }} />
      </div>

      <div style={{ width: 1, height: 22, background: 'var(--k-line)' }} />

      {/* Theme + language now live in the Options modal */}
      <button onClick={onOptions} title={t.options} style={iconBtnStyle}>
        <IcoGear size={17} />
      </button>
    </header>
  )
}

const iconBtnStyle: React.CSSProperties = {
  display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
  width: 36, height: 36, borderRadius: 8,
  border: '1px solid var(--k-line)', background: 'transparent',
  color: 'var(--k-muted)', cursor: 'pointer',
}

const textBtnStyle: React.CSSProperties = {
  display: 'inline-flex', alignItems: 'center', gap: 8,
  height: 36, padding: '0 13px', borderRadius: 8,
  border: '1px solid var(--k-line)', background: 'transparent',
  color: 'var(--k-text)', font: "600 13px 'Inter'", cursor: 'pointer',
}

// navigator.clipboard needs a secure context — absent when the UI is reached
// over plain http on the LAN (http://<ip>:7700), hence the execCommand fallback.
async function copyText(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard) {
      await navigator.clipboard.writeText(text)
      return true
    }
  } catch { /* fall through */ }
  try {
    const ta = document.createElement('textarea')
    ta.value = text
    ta.style.position = 'fixed'
    ta.style.opacity = '0'
    document.body.appendChild(ta)
    ta.select()
    const ok = document.execCommand('copy')
    ta.remove()
    return ok
  } catch {
    return false
  }
}
