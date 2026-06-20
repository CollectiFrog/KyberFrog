import type { ReactNode } from 'react'

interface Props {
  icon: ReactNode
  title: string
  desc: string
}

export function EmptyState({ icon, title, desc }: Props) {
  return (
    <div style={{
      margin: 'auto', textAlign: 'center', maxWidth: 300,
      padding: '40px 0', color: 'var(--k-muted)',
    }}>
      <div style={{ marginBottom: 14, color: 'var(--k-faint)' }}>{icon}</div>
      <div style={{ fontSize: 16, fontWeight: 600, color: 'var(--k-text)', marginBottom: 6 }}>{title}</div>
      <div style={{ fontSize: 13, lineHeight: 1.5 }}>{desc}</div>
    </div>
  )
}
