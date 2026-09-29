import { useQuery } from '@tanstack/react-query'
import { api } from '../api'
import type { LocalMonitor } from '../types'

/** This machine's monitors, for the viewer's output monitor picker (#1). */
export function useMonitors() {
  return useQuery<LocalMonitor[]>({
    queryKey: ['monitors'],
    queryFn: () => api.monitors(),
    staleTime: 10_000,
    refetchOnWindowFocus: true,
  })
}
