import { IcoClose } from '../icons'

interface Props {
  hostname: string
  ip: string
  version: string
  theme: 'dark' | 'light'
  onClose: () => void
}

export function AboutModal({ hostname, ip, version, theme, onClose }: Props) {
  return (
    <div
      onClick={onClose}
      style={{
        position: 'fixed', inset: 0, background: 'rgba(8,11,16,0.6)',
        zIndex: 95, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 20,
      }}
    >
      <div
        onClick={e => e.stopPropagation()}
        className="kf-modal"
        style={{
          width: 'min(420px, 100%)', background: 'var(--k-bg)',
          border: '1px solid var(--k-line)', borderRadius: 14,
          boxShadow: '0 24px 70px rgba(8,11,16,0.5)', overflow: 'hidden',
          color: 'var(--k-text)',
        }}
      >
        <div style={{ padding: '28px 26px 22px', borderBottom: '1px solid var(--k-line)', position: 'relative' }}>
          <button onClick={onClose} style={{ position: 'absolute', top: 14, right: 14, display: 'inline-flex', alignItems: 'center', justifyContent: 'center', width: 32, height: 32, borderRadius: 8, border: '1px solid var(--k-line)', background: 'transparent', color: 'var(--k-text)', cursor: 'pointer' }}>
            <IcoClose size={16} />
          </button>
          <img
            src={theme === 'light' ? '/assets/logo-darkpurple.svg' : '/assets/logo-saffron.svg'}
            alt="KyberFrog"
            width={52}
            height={52}
            style={{ objectFit: 'contain', marginBottom: 14, display: 'block' }}
            onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
          />
          <div style={{ fontSize: 24, fontWeight: 700, letterSpacing: '-0.02em', color: 'var(--k-text)', lineHeight: 1 }}>KyberFrog</div>
          <div style={{ fontSize: 13, color: 'var(--k-muted)', marginTop: 7 }}>Régie vidéo sur réseau local</div>
        </div>

        <div style={{ padding: '18px 26px', display: 'flex', flexDirection: 'column', fontSize: 13, fontFeatureSettings: "'tnum' 1" }}>
          <Row label="Version" value={version} />
          <Row label="Hostname" value={hostname} />
          <Row label="Adresse IP" value={ip} />
          <Row label="Réseau" value="LAN · connecté" accent last />
        </div>

        <div style={{ padding: '4px 26px 22px', display: 'flex', gap: 18, fontSize: 13 }}>
          <a href="https://gitlab.com/kyber-frog/kyberfrog" target="_blank" rel="noreferrer" style={{ color: 'var(--k-accent)', fontWeight: 600, textDecoration: 'none' }}>
            Documentation →
          </a>
          <a href="https://gitlab.com/kyber-frog/kyberfrog/-/issues" target="_blank" rel="noreferrer" style={{ color: 'var(--k-accent)', fontWeight: 600, textDecoration: 'none' }}>
            Support →
          </a>
        </div>
      </div>
    </div>
  )
}

function Row({ label, value, accent, last }: { label: string; value: string; accent?: boolean; last?: boolean }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', padding: '9px 0', borderBottom: last ? 'none' : '1px solid var(--k-line)' }}>
      <span style={{ color: 'var(--k-muted)' }}>{label}</span>
      <span style={{ color: accent ? 'var(--k-accent)' : 'var(--k-text)', fontWeight: 600 }}>{value}</span>
    </div>
  )
}
