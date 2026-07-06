import { useQuery } from '@tanstack/react-query'
import { api } from '../api'
import type { DiscoveredInstance } from '../types'

/**
 * The emitters heard on the LAN via mDNS, for the viewer form's "detected
 * emitters" picker. Polled while the form is open so a machine booting mid-edit
 * shows up; an empty list simply leaves the manual IP entry as the only path.
 */
export function useDiscovered(enabled = true) {
  return useQuery<DiscoveredInstance[]>({
    queryKey: ['discovered'],
    queryFn: api.discovered,
    enabled,
    refetchInterval: 3000,
    staleTime: 0,
    retry: false,
    refetchOnWindowFocus: false,
  })
}
