import { useRef } from 'react'
import { IcoInfo, IcoNetwork, IcoSun, IcoMoon, IcoDownload, IcoUpload, IcoEdit } from '../icons'
import type { Lang, LangStrings } from '../hooks/useLang'

interface Props {
  hostname: string
  ip: string
  online: boolean
  theme: 'dark' | 'light'
  lang: Lang
  t: LangStrings
  activeSetup: string
  setups: string[]
  exportUrl: string
  onToggleTheme: () => void
  onAbout: () => void
  onSetLang: (l: Lang) => void
  onLoadSetup: (name: string) => void
  onSaveAs: () => void
  onImportFile: (file: File) => void
}

export function TopBar({
  hostname, ip, online, theme, lang, t,
  activeSetup, setups, exportUrl,
  onToggleTheme, onAbout, onSetLang, onLoadSetup, onSaveAs, onImportFile,
}: Props) {
  const importRef = useRef<HTMLInputElement>(null)

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
          style={{ flex: 'none', objectFit: 'contain' }}
          onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
        />
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 11, lineHeight: 1 }}>
          <span style={{ fontSize: 17, fontWeight: 700, letterSpacing: '-0.02em', color: 'var(--k-text)' }}>
            KyberFrog
          </span>
          <span style={{ fontSize: 13, fontWeight: 500, color: 'var(--k-muted)', fontFeatureSettings: "'tnum' 1" }}>
            {hostname}
          </span>
        </div>
      </div>

      <div style={{ width: 1, height: 22, background: 'var(--k-line)' }} />

      <div style={{ display: 'flex', alignItems: 'center', gap: 7, color: 'var(--k-muted)', fontSize: 13, fontWeight: 500, fontFeatureSettings: "'tnum' 1" }}>
        <IcoNetwork size={15} />
        {ip}
      </div>

      <div style={{
        display: 'flex', alignItems: 'center', gap: 7,
        padding: '5px 11px', borderRadius: 7,
        background: online ? 'var(--k-accent-soft)' : 'var(--k-danger-soft)',
        border: '1px solid var(--k-line)',
      }}>
        <span style={{
          width: 7, height: 7, borderRadius: '50%',
          background: online ? '#3FB85C' : 'var(--k-danger)',
          animation: online ? 'kf-pulse 2s ease-in-out infinite' : 'none',
        }} />
        <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--k-text)' }}>
          {online ? t.online : t.offline}
        </span>
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

      {/* FR / EN switcher */}
      <div style={{ display: 'flex', alignItems: 'center', border: '1px solid var(--k-line)', borderRadius: 8, overflow: 'hidden' }}>
        {(['fr', 'en'] as Lang[]).map(l => (
          <button
            key={l}
            onClick={() => onSetLang(l)}
            title={l === 'fr' ? 'Français' : 'English'}
            style={{
              height: 36, padding: '0 11px',
              border: 'none',
              background: lang === l ? 'var(--k-accent-soft)' : 'transparent',
              color: lang === l ? 'var(--k-text)' : 'var(--k-muted)',
              font: "600 12px 'Inter'",
              cursor: 'pointer',
              textTransform: 'uppercase',
            }}
          >
            {l}
          </button>
        ))}
      </div>

      <button onClick={onToggleTheme} title={t.themeTitle} style={iconBtnStyle}>
        {theme === 'dark' ? <IcoSun size={18} /> : <IcoMoon size={18} />}
      </button>

      <button onClick={onAbout} style={textBtnStyle}>
        <IcoInfo size={16} />
        {t.about}
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
