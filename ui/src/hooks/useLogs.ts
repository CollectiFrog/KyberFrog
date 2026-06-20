import { useState, useEffect, useRef, useCallback } from 'react'
import { api } from '../api'
import type { LogEntry, LogSourceId } from '../types'

function parseLogLine(raw: string, index: number): LogEntry | null {
  if (!raw.trim()) return null
  // flexi_logger format: "2026-06-20 12:01:03.123 [INFO ] kyberfrog ..."
  const m = raw.match(/^(\d{2}:\d{2}:\d{2})|\d{4}-\d{2}-\d{2}\s+(\d{2}:\d{2}:\d{2})/)
  const levelM = raw.match(/\[(INFO|WARN|ERROR|DEBUG)\s*\]/)
  const level = (levelM?.[1] ?? 'INFO') as LogEntry['level']
  const ts = m?.[1] ?? m?.[2] ?? ''

  const parts = raw.split(/\s+/)
  const msgStart = parts.findIndex(p => ['INFO', 'WARN', 'ERROR', 'DEBUG'].some(l => p.includes(l)))
  const msg = msgStart >= 0 ? parts.slice(msgStart + 1).join(' ') : raw.trim()

  return { id: `log-${index}-${Date.now()}`, ts, level, src: '', msg }
}

export function useLogs(source: LogSourceId, sourceKind: 'app' | 'tx' | 'viewer', sourceName: string) {
  const [entries, setEntries] = useState<LogEntry[]>([])
  const [live, setLive] = useState(true)
  const [paused, setPaused] = useState(false)
  const lastRawRef = useRef('')
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const fetchLogs = useCallback(async () => {
    if (paused) return
    try {
      let raw = ''
      if (sourceKind === 'app') raw = await api.logsApp()
      else if (sourceKind === 'tx') raw = await api.logsTransmitter(sourceName)
      else raw = await api.logsViewer(sourceName)

      if (raw === lastRawRef.current) return
      lastRawRef.current = raw

      const lines = raw.split('\n')
      const parsed: LogEntry[] = []
      lines.forEach((line, i) => {
        const e = parseLogLine(line, i)
        if (e) parsed.push(e)
      })
      setEntries(parsed)
    } catch { /* ignore — backend may not be available */ }
  }, [sourceKind, sourceName, paused])

  useEffect(() => {
    setEntries([])
    lastRawRef.current = ''
  }, [source])

  useEffect(() => {
    if (!live) {
      if (intervalRef.current) clearInterval(intervalRef.current)
      return
    }
    void fetchLogs()
    intervalRef.current = setInterval(fetchLogs, 2000)
    return () => { if (intervalRef.current) clearInterval(intervalRef.current) }
  }, [live, fetchLogs])

  const clear = () => setEntries([])
  const toggleLive = () => setLive(l => !l)
  const togglePause = () => setPaused(p => !p)

  const openFile = () => {
    let path = 'kyberfrog.log'
    if (sourceKind === 'tx') path = `${sourceName}-kycontroller.log`
    else if (sourceKind === 'viewer') path = `kyclient-${sourceName}.log`
    fetch(`/logs/open?file=${encodeURIComponent(path)}`, { method: 'POST' }).catch(() => { /* noop */ })
  }

  return { entries, live, paused, clear, toggleLive, togglePause, openFile }
}
