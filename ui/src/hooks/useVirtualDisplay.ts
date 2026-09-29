import { useQuery } from '@tanstack/react-query'
import { api } from '../api'
import type { VirtualDisplayAvailability } from '../types'

/** Whether this machine can make up a screen for a screen transmitter (#54). */
export function useVirtualDisplay(enabled: boolean) {
  return useQuery<VirtualDisplayAvailability>({
    queryKey: ['virtual-display'],
    queryFn: () => api.virtualDisplay(),
    enabled,
    staleTime: 10_000,
  })
}
