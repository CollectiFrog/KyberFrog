import { useQuery } from '@tanstack/react-query'
import { api } from '../api'
import type { DecklinkFormat } from '../types'

/**
 * Capture modes a specific DeckLink device advertises, for the format_code
 * picker. Enabled only once a device is chosen (from `useDecklinkInputs`).
 */
export function useDecklinkFormats(device: string | null, enabled = true) {
  return useQuery<DecklinkFormat[]>({
    queryKey: ['decklink-formats', device],
    queryFn: () => api.decklinkFormats(device!),
    enabled: enabled && !!device,
    staleTime: 10_000,
    refetchOnWindowFocus: false,
  })
}
