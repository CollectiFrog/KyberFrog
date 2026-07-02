import { useQuery } from '@tanstack/react-query'
import { api } from '../api'
import type { DisplayInfo } from '../types'

/**
 * Enumerate a remote emitter's displays for the viewer screen picker. Enabled
 * only once `server` is set; retries are off so an unreachable/older emitter
 * surfaces quickly and the form falls back to a manual index input.
 */
export function useDisplays(server: string, port: number, enabled = true) {
  return useQuery<DisplayInfo[]>({
    queryKey: ['displays', server, port],
    queryFn: () => api.displays(server, port),
    enabled: enabled && server.trim().length > 0 && port > 0,
    retry: false,
    staleTime: 10_000,
    refetchOnWindowFocus: false,
  })
}
