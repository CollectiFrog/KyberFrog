import { IcoInfo, IcoNetwork, IcoSun, IcoMoon } from '../icons'

interface Props {
  hostname: string
  ip: string
  online: boolean
  theme: 'dark' | 'light'
  onToggleTheme: () => void
  onAbout: () => void
}

export function TopBar({ hostname, ip, online, theme, onToggleTheme, onAbout }: Props) {
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
          width: 8, height: 8, borderRadius: '50%',
          background: online ? '#22c55e' : 'var(--k-danger)',
          boxShadow: online ? '0 0 6px 1px #22c55e99' : 'none',
          animation: online ? 'kf-pulse 2s ease-in-out infinite' : 'none',
        }} />
        <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--k-text)' }}>
          {online ? 'En ligne' : 'Hors ligne'}
        </span>
      </div>

      <div style={{ flex: 1 }} />

      <button
        onClick={onToggleTheme}
        title="Thème clair / sombre"
        style={iconBtnStyle}
      >
        {theme === 'dark' ? <IcoSun size={18} /> : <IcoMoon size={18} />}
      </button>

      <button onClick={onAbout} style={textBtnStyle}>
        <IcoInfo size={16} />
        À propos
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
  height: 36, padding: '0 14px', borderRadius: 8,
  border: '1px solid var(--k-line)', background: 'transparent',
  color: 'var(--k-text)', font: "600 13px 'Inter'", cursor: 'pointer',
}
