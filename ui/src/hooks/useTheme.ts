import { useState, useEffect } from 'react'

export type Theme = 'dark' | 'light' | 'frog'
type PersistedTheme = 'dark' | 'light'

const STORAGE_KEY = 'kf-theme'

function getStored(): PersistedTheme {
  try {
    const v = localStorage.getItem(STORAGE_KEY)
    if (v === 'light' || v === 'dark') return v
  } catch { /* ignore */ }
  return 'dark'
}

export function useTheme() {
  const [theme, setThemeState] = useState<Theme>(getStored)

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
    if (theme !== 'frog') {
      try { localStorage.setItem(STORAGE_KEY, theme) } catch { /* ignore */ }
    }
  }, [theme])

  const setTheme = (t: Theme) => setThemeState(t)
  const toggle = () => setThemeState(t => t === 'dark' ? 'light' : 'dark')

  return { theme, toggle, setTheme }
}
