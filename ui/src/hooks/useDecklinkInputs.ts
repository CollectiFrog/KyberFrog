import { useQuery } from '@tanstack/react-query'
import { api } from '../api'

export function useDecklinkInputs(enabled = true) {
  return useQuery<string[]>({
    queryKey: ['decklink-inputs'],
    queryFn: api.decklinkInputs,
    // Enumeration spawns ffmpeg on the backend; poll slower than Spout.
    refetchInterval: 15000,
    enabled,
  })
}
