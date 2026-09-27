/**
 * Shared app/game state used by every feature: config, game status, app info.
 * Feature slices import these hooks; they never redefine them.
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { z } from "zod";
import { EVENTS, useTauriEvent } from "./events";
import { call } from "./ipc";

// ── Schemas (mirror src-tauri/src/base/config.rs and src-tauri/src/game) ──

export const localeSchema = z.enum(["en", "fr"]);
export const fxQualitySchema = z.enum(["off", "low", "high"]);

export const appConfigSchema = z.object({
  rlInstallOverride: z.string().nullable(),
  locale: localeSchema,
  fxQuality: fxQualitySchema,
  fxPauseWhenGameFocused: z.boolean(),
  autoReapplyAfterUpdate: z.boolean(),
  restoreMapsOnGameExit: z.boolean(),
  tracker: z.object({
    platform: z.string().nullable(),
    playerId: z.string().nullable(),
    favorites: z.array(z.object({ platform: z.string(), playerId: z.string() })).default([]),
    /** tracker.gg lookups (MMR, lobby ranks): off until turned on. */
    onlineRatings: z.boolean().default(false),
  }),
  /** The user turned the Stats API off (it is on by default). */
  statsApiOptOut: z.boolean().default(false),
  decalLibraryFolders: z.array(z.string()),
  onboardingDone: z.boolean(),
});
export type AppConfig = z.infer<typeof appConfigSchema>;

export type AppConfigPatch = Partial<Omit<AppConfig, "rlInstallOverride">> & {
  rlInstallOverride?: string | null;
};

export const runningStateSchema = z.object({
  running: z.boolean(),
  withEac: z.boolean(),
  pid: z.number().nullable(),
  foreground: z.boolean(),
});
export type RunningState = z.infer<typeof runningStateSchema>;

export const rlInstallSchema = z.object({
  root: z.string(),
  cookedDir: z.string(),
  modsDir: z.string(),
  configDir: z.string(),
  exeNoEac: z.string(),
  exeEac: z.string(),
  source: z.enum(["manual", "epic", "steam"]),
});
export type RlInstall = z.infer<typeof rlInstallSchema>;

export const gameStatusSchema = z.object({
  install: rlInstallSchema.nullable(),
  running: runningStateSchema,
  build: z.object({ engineSize: z.number(), engineMtime: z.number() }).nullable(),
});
export type GameStatus = z.infer<typeof gameStatusSchema>;

export const appInfoSchema = z.object({
  version: z.string(),
  dataDir: z.string(),
  logsDir: z.string(),
  debug: z.boolean(),
});

// ── Query keys ──

export const qk = {
  config: ["config"] as const,
  game: ["game"] as const,
  appInfo: ["appInfo"] as const,
};

// ── Hooks ──

export function useConfig() {
  return useQuery({
    queryKey: qk.config,
    queryFn: () => call("config_get", {}, appConfigSchema),
    staleTime: Infinity,
  });
}

export function useUpdateConfig() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (patch: AppConfigPatch) => call("config_update", { patch }, appConfigSchema),
    onSuccess: (cfg) => qc.setQueryData(qk.config, cfg),
  });
}

/** Game status; the `running` part is kept live by the `game://status` event. */
export function useGameStatus() {
  const qc = useQueryClient();
  useTauriEvent(EVENTS.gameStatus, runningStateSchema, (running) => {
    qc.setQueryData<GameStatus>(qk.game, (prev) => (prev ? { ...prev, running } : prev));
  });
  return useQuery({
    queryKey: qk.game,
    queryFn: () => call("game_status", {}, gameStatusSchema),
    staleTime: 30_000,
  });
}

export function useSetInstallDir() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (path: string | null) => call("game_set_install_dir", { path }, gameStatusSchema),
    onSuccess: (status) => {
      qc.setQueryData(qk.game, status);
      void qc.invalidateQueries({ queryKey: qk.config });
    },
  });
}

export function useAppInfo() {
  return useQuery({
    queryKey: qk.appInfo,
    queryFn: () => call("app_info", {}, appInfoSchema),
    staleTime: Infinity,
  });
}
