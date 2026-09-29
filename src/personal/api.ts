/**
 * Personal edition only (see docs/PERSONAL_EDITION.md): MMR from tracker.gg.
 * Mirrors src-tauri/src/personal/{ratings,lobby,mmr,commands}.rs.
 * Never import this folder statically from public code: go through a
 * `__ALXS_PERSONAL__ ? lazy(() => import(...)) : null` gate.
 */
import { z } from "zod";
import { call } from "@/lib/ipc";

export const PERSONAL_EVENTS = {
  ratings: "personal://ratings",
} as const;

export const mmrLineSchema = z.object({
  playlist: z.string(),
  baseline: z.number(),
  latest: z.number(),
  tierIcon: z.string().nullable(),
});
export type MmrLine = z.infer<typeof mmrLineSchema>;

export const lobbyRatingSchema = z.object({
  key: z.string(),
  name: z.string(),
  team: z.number(),
  status: z.enum(["pending", "ok", "unavailable"]),
  playlist: z.string().nullable(),
  rating: z.number().nullable(),
  tier: z.string().nullable(),
  division: z.string().nullable(),
  tierIcon: z.string().nullable(),
});
export type LobbyRating = z.infer<typeof lobbyRatingSchema>;

export const ratingsStateSchema = z.object({
  mmr: z.array(mmrLineSchema),
  lobby: z.array(lobbyRatingSchema),
});
export type RatingsState = z.infer<typeof ratingsStateSchema>;

export const PLATFORMS = ["epic", "steam", "psn", "xbl", "switch"] as const;
export type Platform = (typeof PLATFORMS)[number];

export const playlistRatingSchema = z.object({
  playlist: z.string(),
  rating: z.number().nullable(),
  tier: z.string().nullable(),
  division: z.string().nullable(),
  tierIcon: z.string().nullable(),
  matchesPlayed: z.number().nullable(),
});
export type PlaylistRating = z.infer<typeof playlistRatingSchema>;

export const mmrProfileSchema = z.object({
  platform: z.string(),
  player: z.string(),
  displayName: z.string().nullable(),
  playlists: z.array(playlistRatingSchema),
  fetchedAt: z.string(),
});
export type MmrProfile = z.infer<typeof mmrProfileSchema>;

/** The personal fields of `config.tracker` (the public build does not send them). */
export const personalTrackerSchema = z.object({
  playerId: z.string().nullable().default(null),
  platform: z.string().nullable().default(null),
  favorites: z.array(z.object({ platform: z.string(), playerId: z.string() })).default([]),
  /** tracker.gg lookups (MMR, lobby ranks): off until turned on. */
  onlineRatings: z.boolean().default(false),
});
export type PersonalTracker = z.infer<typeof personalTrackerSchema>;

export const personalApi = {
  ratings: () => call("ratings_state", {}, ratingsStateSchema),
  mmrLookup: (platform: Platform, player: string) =>
    call("mmr_lookup", { platform, player }, mmrProfileSchema),
};

/** Most-moved playlist of the session (for compact displays). */
export function topMmrDelta(lines: readonly MmrLine[]): (MmrLine & { delta: number }) | null {
  const withDelta = lines.map((l) => ({ ...l, delta: l.latest - l.baseline }));
  if (!withDelta.length) return null;
  return withDelta.reduce((best, l) => (Math.abs(l.delta) > Math.abs(best.delta) ? l : best));
}

/** Average rating of the rated players of a team. */
export function teamAverage(players: LobbyRating[]): number | null {
  const rated = players.filter((p) => p.rating != null);
  if (rated.length === 0) return null;
  return Math.round(rated.reduce((sum, p) => sum + (p.rating ?? 0), 0) / rated.length);
}
