import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { notify } from "@/components/ui/toast";
import { qk } from "@/lib/game";
import { type DecalApplyRequest, decalsApi } from "./api";

export const decalKeys = {
  library: ["decals", "library"] as const,
  status: ["decals", "status"] as const,
};

export function useDecalLibrary() {
  return useQuery({ queryKey: decalKeys.library, queryFn: decalsApi.library, staleTime: 60_000 });
}

export function useDecalStatus() {
  return useQuery({ queryKey: decalKeys.status, queryFn: decalsApi.status, staleTime: 15_000 });
}

export function useSetDecalFolders() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (folders: string[]) => decalsApi.setFolders(folders),
    onSuccess: (packs) => {
      qc.setQueryData(decalKeys.library, packs);
      void qc.invalidateQueries({ queryKey: decalKeys.status });
      void qc.invalidateQueries({ queryKey: qk.config });
    },
    onError: notify.error,
  });
}

export function useApplyDecal() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (req: DecalApplyRequest) => decalsApi.apply(req),
    onSuccess: (status) => qc.setQueryData(decalKeys.status, status),
    onError: notify.error,
  });
}

/** Copies the AlphaConsole packs into the app's library (optionally converted). */
export function useImportAlphaConsole() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (convert: boolean) => decalsApi.importAlphaConsole(convert),
    onSettled: () => {
      void qc.invalidateQueries({ queryKey: decalKeys.library });
      void qc.invalidateQueries({ queryKey: decalKeys.status });
    },
    onError: notify.error,
  });
}

export function useRemoveDecal() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => decalsApi.remove(),
    onSuccess: (status) => qc.setQueryData(decalKeys.status, status),
    onError: notify.error,
  });
}
