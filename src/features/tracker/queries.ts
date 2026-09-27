import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { notify } from "@/components/ui/toast";
import { EVENTS, useTauriEvent } from "@/lib/events";
import { type Platform, type SessionState, type StatsStatus, sessionStateSchema, trackerApi } from "./api";

export const trackerKeys = {
  status: ["tracker", "stats-status"] as const,
  session: ["tracker", "session"] as const,
  mmr: (platform: string, player: string) => ["tracker", "mmr", platform, player] as const,
};

export function useStatsStatus() {
  return useQuery({ queryKey: trackerKeys.status, queryFn: trackerApi.statsStatus, refetchInterval: 5_000 });
}

/** Tracker session, kept live by `tracker://session`. */
export function useTrackerSession() {
  const qc = useQueryClient();
  useTauriEvent(EVENTS.trackerSession, sessionStateSchema, (s) => qc.setQueryData(trackerKeys.session, s));
  return useQuery({ queryKey: trackerKeys.session, queryFn: trackerApi.session });
}

function useSetStatus() {
  const qc = useQueryClient();
  return (s: StatsStatus) => qc.setQueryData(trackerKeys.status, s);
}

export function useToggleStats() {
  const setStatus = useSetStatus();
  return useMutation({
    mutationFn: (enable: boolean) => (enable ? trackerApi.statsEnable() : trackerApi.statsDisable()),
    onSuccess: setStatus,
    onError: notify.error,
  });
}

export function useToggleOverlay() {
  const setStatus = useSetStatus();
  return useMutation({
    mutationFn: (open: boolean) => (open ? trackerApi.overlayOpen() : trackerApi.overlayClose()),
    onSuccess: setStatus,
    onError: notify.error,
  });
}

export function useResetSession() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: trackerApi.resetSession,
    onSuccess: (s: SessionState) => qc.setQueryData(trackerKeys.session, s),
    onError: notify.error,
  });
}

export function useMmrLookup(platform: Platform | null, player: string | null) {
  return useQuery({
    queryKey: trackerKeys.mmr(platform ?? "", player ?? ""),
    queryFn: () => trackerApi.mmrLookup(platform as Platform, player as string),
    enabled: Boolean(platform && player),
    staleTime: 5 * 60_000,
    retry: false,
  });
}
