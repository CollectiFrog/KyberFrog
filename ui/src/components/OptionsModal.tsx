import { IcoClose } from '../icons'
import type { Theme } from '../hooks/useTheme'
import type { Lang, LangStrings } from '../hooks/useLang'
import type { EncoderId, EncoderInfo } from '../types'

interface Props {
  hostname: string
  ip: string
  version: string
  theme: Theme
  lang: Lang
  t: LangStrings
  onSetTheme: (t: 'dark' | 'light') => void
  onSetLang: (l: Lang) => void
  /** Absent until the first status poll lands. */
  encoder?: EncoderInfo
  onSetEncoder: (id: EncoderId) => void
  onClose: () => void
}

const ENCODER_NAMES: Record<Exclude<EncoderId, 'auto'>, string> = {
  x264: 'x264 (CPU)',
  amf: 'AMF',
  nvenc: 'NVENC',
  qsv: 'Quick Sync',
}

/** "AMF — AMD Radeon RX 7800 XT", "x264 (CPU)", "Auto — AMF (AMD Radeon …)". */
function encoderLabel(id: EncoderId, info: EncoderInfo, available: boolean, t: LangStrings): string {
  if (id === 'auto') {
    const resolved = info.resolved === 'x264' ? ENCODER_NAMES.x264 : `${ENCODER_NAMES[info.resolved]}${info.gpu ? ` (${info.gpu})` : ''}`
    return `${t.encoderAuto} — ${resolved}`
  }
  if (id === 'x264') return ENCODER_NAMES.x264
  return `${ENCODER_NAMES[id]} — ${available && info.gpu ? info.gpu : t.encoderUnavailable}`
}

export function OptionsModal({ hostname, ip, version, theme, lang, t, onSetTheme, onSetLang, encoder, onSetEncoder, onClose }: Props) {
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
          <div style={{ fontSize: 13, color: 'var(--k-muted)', marginTop: 7 }}>{t.tagline}</div>
        </div>

        {/* Preferences: theme + language, persisted machine-side */}
        <div style={{ padding: '18px 26px', borderBottom: '1px solid var(--k-line)', display: 'flex', flexDirection: 'column', fontSize: 13 }}>
          <PrefRow label={t.prefTheme}>
            <Segmented
              value={theme === 'frog' ? null : theme}
              options={[{ value: 'light', label: t.themeLight }, { value: 'dark', label: t.themeDark }]}
              onChange={onSetTheme}
            />
          </PrefRow>
          <PrefRow label={t.prefLang} last={!encoder}>
            <Segmented
              value={lang}
              options={[{ value: 'fr', label: 'FR' }, { value: 'en', label: 'EN' }]}
              onChange={onSetLang}
            />
          </PrefRow>
          {encoder && (
            <>
              <PrefRow label={t.prefEncoder} last>
                <select
                  value={encoder.choice}
                  onChange={e => onSetEncoder(e.target.value as EncoderId)}
                  style={{ height: 32, padding: '0 10px', background: 'var(--k-input)', border: '1px solid var(--k-line)', borderRadius: 8, color: 'var(--k-text)', font: "600 12px 'Inter'", cursor: 'pointer', maxWidth: 250, outline: 'none' }}
                >
                  {encoder.options.map(o => (
                    <option key={o.id} value={o.id} disabled={!o.available && o.id !== encoder.choice}>
                      {encoderLabel(o.id, encoder, o.available, t)}
                    </option>
                  ))}
                </select>
              </PrefRow>
              <div style={{ color: 'var(--k-faint)', fontSize: 12, marginTop: -2 }}>{t.encoderHint}</div>
            </>
          )}
        </div>

        <div style={{ padding: '18px 26px', display: 'flex', flexDirection: 'column', fontSize: 13, fontFeatureSettings: "'tnum' 1" }}>
          <Row label={t.verLabel} value={version} />
          <Row label={t.hostLabel} value={hostname} />
          <Row label={t.ipLabel} value={ip} />
          <Row label={t.netLabel} value={t.netValue} accent last />
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

function PrefRow({ label, last, children }: { label: string; last?: boolean; children: React.ReactNode }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '8px 0', borderBottom: last ? 'none' : '1px solid var(--k-line)' }}>
      <span style={{ color: 'var(--k-muted)' }}>{label}</span>
      {children}
    </div>
  )
}

function Segmented<V extends string>({ value, options, onChange }: {
  value: V | null
  options: { value: V; label: string }[]
  onChange: (v: V) => void
}) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', border: '1px solid var(--k-line)', borderRadius: 8, overflow: 'hidden' }}>
      {options.map(o => (
        <button
          key={o.value}
          onClick={() => onChange(o.value)}
          aria-pressed={value === o.value}
          style={{
            height: 32, padding: '0 14px',
            border: 'none',
            background: value === o.value ? 'var(--k-accent-soft)' : 'transparent',
            color: value === o.value ? 'var(--k-text)' : 'var(--k-muted)',
            font: "600 12px 'Inter'",
            cursor: 'pointer',
          }}
        >
          {o.label}
        </button>
      ))}
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
