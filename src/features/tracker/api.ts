import { z } from "zod";
import { call } from "@/lib/ipc";

// Mirrors src-tauri/src/stats/{tracker,commands}.rs (MMR: src/personal).

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
  /** Missing on matches recorded before 2026-09-29. */
  startedAt: z.string().nullable().optional(),
  playlistId: z.number().nullable().optional(),
  teamSize: z.number().nullable().optional(),
});
export type MatchRecord = z.infer<typeof matchRecordSchema>;

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
  connection: connectionSchema,
  youName: z.string().nullable(),
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

export const trackerApi = {
  statsStatus: () => call("stats_status", {}, statsStatusSchema),
  statsEnable: () => call("stats_enable", {}, statsStatusSchema),
  statsDisable: () => call("stats_disable", {}, statsStatusSchema),
  session: () => call("tracker_session", {}, sessionStateSchema),
  resetSession: () => call("tracker_reset", {}, sessionStateSchema),
  history: () => call("tracker_history", {}, z.array(matchRecordSchema)),
  overlayOpen: () => call("overlay_open", {}, statsStatusSchema),
  overlayClose: () => call("overlay_close", {}, statsStatusSchema),
};

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
