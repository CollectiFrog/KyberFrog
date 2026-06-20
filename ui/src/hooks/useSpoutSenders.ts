import { useQuery } from '@tanstack/react-query'
import { api } from '../api'
import type { SpoutSendersPayload } from '../types'

export function useSpoutSenders(enabled = true) {
  return useQuery<SpoutSendersPayload>({
    queryKey: ['spout-senders'],
    queryFn: api.spoutSenders,
    refetchInterval: 5000,
    enabled,
  })
}
