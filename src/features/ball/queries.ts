import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { notify } from "@/components/ui/toast";
import { ballApi } from "./api";

export const ballKeys = {
  library: ["ball", "library"] as const,
  status: ["ball", "status"] as const,
};

export function useBallLibrary() {
  return useQuery({ queryKey: ballKeys.library, queryFn: ballApi.library, staleTime: 60_000 });
}

export function useBallStatus() {
  return useQuery({ queryKey: ballKeys.status, queryFn: ballApi.status, staleTime: 15_000 });
}

export function useApplyBall() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (packId: string) => ballApi.apply(packId),
    onSuccess: (status) => qc.setQueryData(ballKeys.status, status),
    onError: notify.error,
  });
}

export function useRemoveBall() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => ballApi.remove(),
    onSuccess: (status) => qc.setQueryData(ballKeys.status, status),
    onError: notify.error,
  });
}

/** Copies AlphaConsole's ball packs into the app's library. */
export function useImportBalls() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => ballApi.importAlphaConsole(),
    onSettled: () => {
      void qc.invalidateQueries({ queryKey: ballKeys.library });
      void qc.invalidateQueries({ queryKey: ballKeys.status });
    },
    onError: notify.error,
  });
}
