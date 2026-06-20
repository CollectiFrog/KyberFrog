import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { StatusPayload } from '../types'

export function useStatus() {
  return useQuery<StatusPayload>({
    queryKey: ['status'],
    queryFn: api.status,
    refetchInterval: 2000,
    staleTime: 0,
  })
}

function useMutateStatus<TArg>(fn: (arg: TArg) => Promise<StatusPayload>) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: fn,
    onSuccess: (data) => qc.setQueryData(['status'], data),
    onError: () => qc.invalidateQueries({ queryKey: ['status'] }),
  })
}

export function useStartTransmitter() {
  return useMutateStatus(api.startTransmitter)
}
export function useStopTransmitter() {
  return useMutateStatus(api.stopTransmitter)
}
export function useRestartTransmitter() {
  return useMutateStatus(api.restartTransmitter)
}
export function useDeleteTransmitter() {
  return useMutateStatus(api.deleteTransmitter)
}
export function useAddTransmitter() {
  return useMutateStatus(api.addTransmitter)
}

export function useStartViewer() {
  return useMutateStatus(api.startViewer)
}
export function useStopViewer() {
  return useMutateStatus(api.stopViewer)
}
export function useRestartViewer() {
  return useMutateStatus(api.restartViewer)
}
export function useDeleteViewer() {
  return useMutateStatus(api.deleteViewer)
}
export function useCreateViewer() {
  return useMutateStatus(api.createViewer)
}
export function useUpdateViewer() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, form }: { id: string; form: Parameters<typeof api.updateViewer>[1] }) =>
      api.updateViewer(id, form),
    onSuccess: (data) => qc.setQueryData(['status'], data),
    onError: () => qc.invalidateQueries({ queryKey: ['status'] }),
  })
}
