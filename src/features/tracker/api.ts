import { z } from "zod";
import { call } from "@/lib/ipc";

// Mirrors src-tauri/src/stats/{tracker,commands,mmr}.rs

export const connectionSchema = z.enum(["idle", "connecting", "connected", "unreachable"]);
export type Connection = z.infer<typeof connectionSchema>;

export const playerLineSchema = z.object({
  name: z.string(),
  primaryId: z.string().nullable().optional(),
  team: z.number(),
  score: z.number(),
  goals: z.number(),
  assists: z.number(),
  saves: z.number(),
  shots: z.number(),
  demos: z.number(),
  touches: z.number(),
});
export type PlayerLine = z.infer<typeof playerLineSchema>;

export const liveMatchSchema = z.object({
  guid: z.string().nullable(),
  arena: z.string().nullable(),
  timeSeconds: z.number().nullable(),
  overtime: z.boolean(),
  replay: z.boolean(),
  blueScore: z.number(),
  orangeScore: z.number(),
  yourTeam: z.number().nullable(),
  you: playerLineSchema.nullable(),
  players: z.array(playerLineSchema),
  startedAt: z.string().nullable(),
  ended: z.boolean(),
});
export type LiveMatch = z.infer<typeof liveMatchSchema>;

export const matchRecordSchema = z.object({
  guid: z.string().nullable(),
  arena: z.string().nullable(),
  endedAt: z.string(),
  result: z.enum(["win", "loss", "unknown"]),
  yourTeam: z.number().nullable(),
  blueScore: z.number(),
  orangeScore: z.number(),
  overtime: z.boolean(),
  you: playerLineSchema.nullable(),
  mvp: z.boolean(),
});
export type MatchRecord = z.infer<typeof matchRecordSchema>;

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

export const sessionStateSchema = z.object({
  startedAt: z.string(),
  wins: z.number(),
  losses: z.number(),
  streak: z.number(),
  bestStreak: z.number(),
  totals: z.object({
    goals: z.number(),
    assists: z.number(),
    saves: z.number(),
    shots: z.number(),
    demos: z.number(),
    score: z.number(),
    mvps: z.number(),
  }),
  matches: z.array(matchRecordSchema),
  live: liveMatchSchema.nullable(),
  mmr: z.array(mmrLineSchema),
  connection: connectionSchema,
  youName: z.string().nullable(),
  lobby: z.array(lobbyRatingSchema).default([]),
});
export type SessionState = z.infer<typeof sessionStateSchema>;

export const statsStatusSchema = z.object({
  iniPath: z.string().nullable(),
  settings: z.object({
    port: z.number().nullable(),
    webPort: z.number().nullable(),
    packetSendRate: z.number().nullable(),
  }),
  enabled: z.boolean(),
  connection: connectionSchema,
  gameRunning: z.boolean(),
  overlayOpen: z.boolean(),
  problem: z.string().nullable(),
});
export type StatsStatus = z.infer<typeof statsStatusSchema>;

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

export const trackerApi = {
  statsStatus: () => call("stats_status", {}, statsStatusSchema),
  statsEnable: () => call("stats_enable", {}, statsStatusSchema),
  statsDisable: () => call("stats_disable", {}, statsStatusSchema),
  session: () => call("tracker_session", {}, sessionStateSchema),
  resetSession: () => call("tracker_reset", {}, sessionStateSchema),
  mmrLookup: (platform: Platform, player: string) =>
    call("mmr_lookup", { platform, player }, mmrProfileSchema),
  overlayOpen: () => call("overlay_open", {}, statsStatusSchema),
  overlayClose: () => call("overlay_close", {}, statsStatusSchema),
};

/** Most-moved playlist of the session (for compact displays). */
export function topMmrDelta(lines: readonly MmrLine[]): (MmrLine & { delta: number }) | null {
  const withDelta = lines.map((l) => ({ ...l, delta: l.latest - l.baseline }));
  if (!withDelta.length) return null;
  return withDelta.reduce((best, l) => (Math.abs(l.delta) > Math.abs(best.delta) ? l : best));
}

/** `Stadium_P` / `cs_day_p` → `Stadium` / `Cs Day`. */
export function arenaLabel(arena: string | null | undefined): string {
  if (!arena) return "—";
  return arena
    .replace(/_P$/i, "")
    .split("_")
    .filter(Boolean)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
    .join(" ");
}

export function formatClock(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined) return "--:--";
  const s = Math.max(0, Math.floor(seconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}
