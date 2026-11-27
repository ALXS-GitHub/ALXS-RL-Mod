import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { notify } from "@/components/ui/toast";
import { EVENTS, useTauriEvent } from "@/lib/events";
import {
  type DownloadProgress,
  downloadProgressSchema,
  type MapPatch,
  type MapSort,
  mapsApi,
  type RemoteSource,
} from "./api";

export const mapsKeys = {
  all: ["maps"] as const,
  list: ["maps", "list"] as const,
  session: ["maps", "session"] as const,
  textures: ["maps", "textures"] as const,
  repairStatus: (version: number | null) => ["maps", "repairStatus", version] as const,
  browse: (source: RemoteSource, query: string, page: number, sort: MapSort) =>
    ["maps", "browse", source, query, page, sort] as const,
};

export function useMaps() {
  return useQuery({ queryKey: mapsKeys.list, queryFn: mapsApi.list });
}

export function useRepairMappings() {
  return useQuery({ queryKey: ["mapsRepairMappings"], queryFn: mapsApi.repairMappings, staleTime: Infinity });
}

/** What mapping `version` (null = latest) would fix in every library map. */
export function useRepairStatus(version: number | null = null) {
  return useQuery({ queryKey: mapsKeys.repairStatus(version), queryFn: () => mapsApi.repairStatus(version) });
}

function useInvalidateAfterRepair() {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: mapsKeys.all });
    void qc.invalidateQueries({ queryKey: ["multi", "check"] });
  };
}

export function useRepairMap() {
  const invalidate = useInvalidateAfterRepair();
  return useMutation({
    mutationFn: ({ id, version }: { id: string; version: number | null }) => mapsApi.repair(id, version),
    onSuccess: invalidate,
    onError: notify.error,
  });
}

export function useUndoRepair() {
  const invalidate = useInvalidateAfterRepair();
  return useMutation({ mutationFn: mapsApi.repairUndo, onSuccess: invalidate, onError: notify.error });
}

export function useTexturesStatus() {
  return useQuery({ queryKey: mapsKeys.textures, queryFn: mapsApi.texturesStatus });
}

/** Explicit download of the workshop textures pack. */
export function useInstallTextures() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: mapsApi.texturesInstall,
    onSuccess: (s) => qc.setQueryData(mapsKeys.textures, s),
    onError: notify.error,
  });
}

export function useRemoveTextures() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: mapsApi.texturesRemove,
    onSuccess: (s) => qc.setQueryData(mapsKeys.textures, s),
    onError: notify.error,
  });
}

export function useMapSession() {
  return useQuery({ queryKey: mapsKeys.session, queryFn: mapsApi.session, refetchInterval: 10_000 });
}

export function useBrowse(source: RemoteSource, query: string, page: number, sort: MapSort) {
  return useQuery({
    queryKey: mapsKeys.browse(source, query, page, sort),
    queryFn: () => mapsApi.browse(source, query, page, sort),
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
