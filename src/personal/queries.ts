import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTauriEvent } from "@/lib/events";
import { PERSONAL_EVENTS, type Platform, personalApi, ratingsStateSchema } from "./api";

export const personalKeys = {
  ratings: ["personal", "ratings"] as const,
  mmr: (platform: string, player: string) => ["personal", "mmr", platform, player] as const,
};

/** Your MMR lines and the lobby ratings, kept live by `personal://ratings`. */
export function useRatings() {
  const qc = useQueryClient();
  useTauriEvent(PERSONAL_EVENTS.ratings, ratingsStateSchema, (s) => qc.setQueryData(personalKeys.ratings, s));
  return useQuery({ queryKey: personalKeys.ratings, queryFn: personalApi.ratings });
}

export function useMmrLookup(platform: Platform | null, player: string | null) {
  return useQuery({
    queryKey: personalKeys.mmr(platform ?? "", player ?? ""),
    queryFn: () => personalApi.mmrLookup(platform as Platform, player as string),
    enabled: Boolean(platform && player),
    staleTime: 5 * 60_000,
    retry: false,
  });
}
