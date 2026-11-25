import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { notify } from "@/components/ui/toast";
import { mapsKeys } from "@/features/maps/queries";
import { EVENTS, useTauriEvent } from "@/lib/events";
import { multiApi, multiSessionEventSchema } from "./api";

export const multiKeys = {
  status: ["multi", "status"] as const,
  session: ["multi", "session"] as const,
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
  return useMutation({ mutationFn: multiApi.host, onSuccess: set, onError: notify.error });
}

export function useJoin() {
  const set = useSetSession();
  return useMutation({ mutationFn: multiApi.join, onSuccess: set, onError: notify.error });
}

export function useDecodeInvite(code: string) {
  const trimmed = code.trim();
  return useQuery({
    queryKey: ["multi", "decode", trimmed],
    queryFn: () => multiApi.decode(trimmed),
    enabled: trimmed.length > 12,
    retry: false,
  });
}

export function useCancelSession() {
  const set = useSetSession();
  return useMutation({ mutationFn: multiApi.cancel, onSuccess: () => set(null), onError: notify.error });
}
