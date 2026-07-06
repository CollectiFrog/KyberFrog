import { useQuery } from '@tanstack/react-query'
import { api } from '../api'

export function useCameras(enabled = true) {
  return useQuery<string[]>({
    queryKey: ['cameras'],
    queryFn: api.cameras,
    // Enumeration spawns ffmpeg on the backend; poll slower than Spout.
    refetchInterval: 15000,
    enabled,
  })
}
