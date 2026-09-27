import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { notify } from "@/components/ui/toast";
import { EVENTS, useTauriEvent } from "@/lib/events";
import {
  type DownloadProgress,
  downloadProgressSchema,
  type MapPatch,
  mapsApi,
  type RemoteSource,
} from "./api";

export const mapsKeys = {
  all: ["maps"] as const,
  list: ["maps", "list"] as const,
  session: ["maps", "session"] as const,
  browse: (source: RemoteSource, query: string, page: number) =>
    ["maps", "browse", source, query, page] as const,
};

export function useMaps() {
  return useQuery({ queryKey: mapsKeys.list, queryFn: mapsApi.list });
}

export function useMapSession() {
  return useQuery({ queryKey: mapsKeys.session, queryFn: mapsApi.session, refetchInterval: 10_000 });
}

export function useBrowse(source: RemoteSource, query: string, page: number) {
  return useQuery({
    queryKey: mapsKeys.browse(source, query, page),
    queryFn: () => mapsApi.browse(source, query, page),
    placeholderData: keepPreviousData,
    staleTime: 5 * 60_000,
  });
}

function useInvalidateMaps() {
  const qc = useQueryClient();
  return () => qc.invalidateQueries({ queryKey: mapsKeys.all });
}

export function useImportMaps() {
  const invalidate = useInvalidateMaps();
  return useMutation({ mutationFn: mapsApi.import, onSuccess: invalidate, onError: notify.error });
}

export function useUpdateMap() {
  const invalidate = useInvalidateMaps();
  return useMutation({
    mutationFn: ({ id, patch }: { id: string; patch: MapPatch }) => mapsApi.update(id, patch),
    onSuccess: invalidate,
    onError: notify.error,
  });
}

export function useDeleteMap() {
  const invalidate = useInvalidateMaps();
  return useMutation({ mutationFn: mapsApi.remove, onSuccess: invalidate, onError: notify.error });
}

export function useActivateMap() {
  const invalidate = useInvalidateMaps();
  return useMutation({
    mutationFn: ({ id, target }: { id: string; target: string | null }) => mapsApi.activate(id, target),
    onSuccess: invalidate,
    onError: notify.error,
  });
}

export function usePlayOffline() {
  const invalidate = useInvalidateMaps();
  return useMutation({ mutationFn: mapsApi.playOffline, onSuccess: invalidate, onError: notify.error });
}

export function useDeactivateMap() {
  const invalidate = useInvalidateMaps();
  return useMutation({ mutationFn: mapsApi.deactivate, onSuccess: invalidate, onError: notify.error });
}

export function useDownloadMap() {
  const invalidate = useInvalidateMaps();
  return useMutation({
    mutationFn: ({ source, remoteId }: { source: RemoteSource; remoteId: string }) =>
      mapsApi.download(source, remoteId),
    onSuccess: invalidate,
    onError: notify.error,
  });
}

/** Live download progress keyed by `<source>:<remoteId>`. */
export function useDownloadProgress() {
  const [progress, setProgress] = useState<Record<string, DownloadProgress>>({});
  useTauriEvent(EVENTS.mapsDownload, downloadProgressSchema, (p) => {
    setProgress((prev) => {
      const key = `${p.source}:${p.remoteId}`;
      if (p.phase === "done" || p.phase === "failed") {
        const { [key]: _removed, ...rest } = prev;
        return rest;
      }
      return { ...prev, [key]: p };
    });
  });
  return progress;
}
