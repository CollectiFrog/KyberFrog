interface Props {
  kind: string
  name: string
  onConfirm: () => void
  onCancel: () => void
}

export function ConfirmDialog({ kind, name, onConfirm, onCancel }: Props) {
  return (
    <div
      onClick={onCancel}
      style={{
        position: 'fixed', inset: 0, background: 'rgba(8,11,16,0.6)',
        zIndex: 100, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 20,
      }}
    >
      <div
        onClick={e => e.stopPropagation()}
        className="kf-modal"
        style={{
          width: 'min(380px, 100%)', background: 'var(--k-bg)',
          border: '1px solid var(--k-line)', borderRadius: 12,
          boxShadow: '0 20px 60px rgba(8,11,16,0.5)', padding: '24px 24px 20px',
          color: 'var(--k-text)',
        }}
      >
        <div style={{ fontSize: 17, fontWeight: 700, marginBottom: 10 }}>
          Supprimer {kind}
        </div>
        <div style={{ fontSize: 14, color: 'var(--k-muted)', lineHeight: 1.5, marginBottom: 22 }}>
          Voulez-vous vraiment supprimer <strong style={{ color: 'var(--k-text)' }}>« {name} »</strong> ?
          Cette action est irréversible.
        </div>
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 10 }}>
          <button onClick={onCancel} style={cancelBtn}>Annuler</button>
          <button onClick={onConfirm} style={deleteBtn}>Supprimer</button>
        </div>
      </div>
    </div>
  )
}

const cancelBtn: React.CSSProperties = {
  height: 38, padding: '0 16px', borderRadius: 8,
  border: '1px solid var(--k-line)', background: 'transparent',
  color: 'var(--k-text)', font: "600 13px 'Inter'", cursor: 'pointer',
}

const deleteBtn: React.CSSProperties = {
  height: 38, padding: '0 16px', borderRadius: 8,
  border: 'none', background: 'var(--k-danger)',
  color: '#fff', font: "600 13px 'Inter'", cursor: 'pointer',
}
