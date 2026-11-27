import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { notify } from "@/components/ui/toast";
import { mapsKeys } from "@/features/maps/queries";
import { EVENTS, useTauriEvent } from "@/lib/events";
import { type HostPresetInput, type MatchOptions, multiApi, multiSessionEventSchema } from "./api";

export const multiKeys = {
  status: ["multi", "status"] as const,
  session: ["multi", "session"] as const,
  check: ["multi", "check"] as const,
};

export function useMultiStatus() {
  return useQuery({ queryKey: multiKeys.status, queryFn: multiApi.status, refetchInterval: 8_000 });
}

/** Current session, kept live by `multi://session`. */
export function useMultiSession() {
  const qc = useQueryClient();
  useTauriEvent(EVENTS.multiSession, multiSessionEventSchema, (s) => {
    qc.setQueryData(multiKeys.session, s);
    void qc.invalidateQueries({ queryKey: mapsKeys.all });
    void qc.invalidateQueries({ queryKey: multiKeys.status });
    void qc.invalidateQueries({ queryKey: multiKeys.check });
  });
  return useQuery({ queryKey: multiKeys.session, queryFn: multiApi.session });
}

function useSetSession() {
  const qc = useQueryClient();
  return (data: unknown) => {
    qc.setQueryData(multiKeys.session, data);
    void qc.invalidateQueries({ queryKey: multiKeys.status });
  };
}

export function useBakkesSetup() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: multiApi.bakkesSetup,
    onSuccess: () => qc.invalidateQueries({ queryKey: multiKeys.status }),
    onError: notify.error,
  });
}

export function useBakkesUndo() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: multiApi.bakkesUndo,
    onSuccess: () => qc.invalidateQueries({ queryKey: multiKeys.status }),
    onError: notify.error,
  });
}

export function useHost() {
  const set = useSetSession();
  return useMutation({
    mutationFn: ({ mapId, target, options }: { mapId: string; target: string; options: MatchOptions }) =>
      multiApi.host(mapId, target, options),
    onSuccess: set,
    onError: notify.error,
  });
}

export const hostPresetsKey = ["multi", "hostPresets"] as const;

export function useHostPresets() {
  return useQuery({ queryKey: hostPresetsKey, queryFn: multiApi.presetsList });
}

export function useSaveHostPreset() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (p: HostPresetInput) => multiApi.presetsSave(p),
    onSuccess: () => qc.invalidateQueries({ queryKey: hostPresetsKey }),
    onError: notify.error,
  });
}

export function useDeleteHostPreset() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: multiApi.presetsDelete,
    onSuccess: () => qc.invalidateQueries({ queryKey: hostPresetsKey }),
    onError: notify.error,
  });
}

export function useRpOptions() {
  return useQuery({ queryKey: ["multi", "rpOptions"], queryFn: multiApi.rpOptions, staleTime: 60_000 });
}

export function useInstallRocketPlugin() {
  return useMutation({ mutationFn: multiApi.installRocketPlugin });
}

export function useJoin() {
  const set = useSetSession();
  return useMutation({ mutationFn: multiApi.join, onSuccess: set, onError: notify.error });
}

/** Invite + where its map stands in the library (re-checked after a download). */
export function useInviteCheck(code: string) {
  const trimmed = code.trim();
  return useQuery({
    queryKey: [...multiKeys.check, trimmed],
    queryFn: () => multiApi.check(trimmed),
    enabled: trimmed.length > 12,
    retry: false,
  });
}

export function useCancelSession() {
  const set = useSetSession();
  return useMutation({ mutationFn: multiApi.cancel, onSuccess: () => set(null), onError: notify.error });
}
