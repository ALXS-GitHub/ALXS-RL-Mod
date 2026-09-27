import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { notify } from "@/components/ui/toast";
import { type PaletteDraft, paletteApi } from "./api";

export const paletteKeys = {
  all: ["palette"] as const,
  status: ["palette", "status"] as const,
  stock: ["palette", "stock"] as const,
  list: ["palette", "list"] as const,
};

export function usePaletteStatus() {
  return useQuery({ queryKey: paletteKeys.status, queryFn: paletteApi.status, staleTime: 15_000 });
}

/** Stock colours are read from the game file once per session (cached backend-side too). */
export function useStockPalette(enabled = true) {
  return useQuery({ queryKey: paletteKeys.stock, queryFn: paletteApi.stock, staleTime: Infinity, enabled });
}

export function usePalettes() {
  return useQuery({ queryKey: paletteKeys.list, queryFn: paletteApi.list });
}

export function useSavePalette() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (draft: PaletteDraft) => paletteApi.save(draft),
    onSuccess: () => void qc.invalidateQueries({ queryKey: paletteKeys.list }),
    onError: notify.error,
  });
}

export function useDeletePalette() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => paletteApi.remove(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: paletteKeys.list });
      void qc.invalidateQueries({ queryKey: paletteKeys.status });
    },
    onError: notify.error,
  });
}

export function useApplyPalette() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => paletteApi.apply(id),
    onSuccess: (status) => qc.setQueryData(paletteKeys.status, status),
    onError: notify.error,
  });
}

export function useRestorePalette() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => paletteApi.restore(),
    onSuccess: (status) => qc.setQueryData(paletteKeys.status, status),
    onError: notify.error,
  });
}
