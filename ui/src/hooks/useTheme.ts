import { useState, useEffect } from 'react'

type Theme = 'dark' | 'light'

const STORAGE_KEY = 'kf-theme'

function getStored(): Theme {
  try {
    const v = localStorage.getItem(STORAGE_KEY)
    if (v === 'light' || v === 'dark') return v
  } catch { /* ignore */ }
  return 'dark'
}

export function useTheme() {
  const [theme, setTheme] = useState<Theme>(getStored)

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
    try { localStorage.setItem(STORAGE_KEY, theme) } catch { /* ignore */ }
  }, [theme])

  const toggle = () => setTheme(t => t === 'dark' ? 'light' : 'dark')

  return { theme, toggle, setTheme }
}
