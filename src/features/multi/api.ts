import { z } from "zod";
import { remoteSourceSchema } from "@/features/maps/api";
import { call, callVoid } from "@/lib/ipc";

// Mirrors src-tauri/src/multi/{tailscale,bakkes_cfg,invite,session,commands}.rs

export const tailscaleStateSchema = z.enum([
  "notInstalled",
  "serviceDown",
  "needsLogin",
  "stopped",
  "running",
]);
export type TailscaleState = z.infer<typeof tailscaleStateSchema>;

export const tailscalePeerSchema = z.object({
  name: z.string(),
  ipv4: z.string().nullable(),
  online: z.boolean(),
  os: z.string().nullable(),
});
export type TailscalePeer = z.infer<typeof tailscalePeerSchema>;

export const tailscaleStatusSchema = z.object({
  state: tailscaleStateSchema,
  cliPath: z.string().nullable(),
  selfName: z.string().nullable(),
  selfIpv4: z.string().nullable(),
  tailnet: z.string().nullable(),
  peers: z.array(tailscalePeerSchema),
});
export type TailscaleStatus = z.infer<typeof tailscaleStatusSchema>;

export const bakkesSetupSchema = z.object({
  dataDir: z.string().nullable(),
  injectorPath: z.string().nullable(),
  rocketPluginInstalled: z.boolean(),
  rocketPluginLoaded: z.boolean(),
  rconReady: z.boolean(),
  rconPluginInstalled: z.boolean(),
  rconPort: z.number(),
  injectorRunning: z.boolean(),
});
export type BakkesSetup = z.infer<typeof bakkesSetupSchema>;

export const multiStatusSchema = z.object({
  tailscale: tailscaleStatusSchema,
  bakkes: bakkesSetupSchema,
  gameRunning: z.boolean(),
  gameWithEac: z.boolean(),
  port: z.number(),
});
export type MultiStatus = z.infer<typeof multiStatusSchema>;

export const inviteSchema = z.object({
  v: z.number(),
  host: z.string(),
  port: z.number(),
  hostName: z.string().nullable(),
  map: z.object({
    name: z.string(),
    target: z.string(),
    sha256: z.string().nullable(),
    source: remoteSourceSchema.nullable(),
    remoteId: z.string().nullable(),
  }),
});
export type Invite = z.infer<typeof inviteSchema>;

export const inviteCheckSchema = z.object({
  invite: inviteSchema,
  libraryMapId: z.string().nullable(),
  libraryMapName: z.string().nullable(),
  alreadyOnArena: z.boolean(),
  downloadable: z.boolean(),
});
export type InviteCheck = z.infer<typeof inviteCheckSchema>;

/** Rocket Plugin on bakkesplugins (fallback when the installer cannot start). */
export const ROCKET_PLUGIN_PAGE = "https://bakkesplugins.com/plugin/26";
export const BAKKESMOD_DOWNLOAD = "https://github.com/bakkesmodorg/BakkesModInjectorCpp/releases/latest";
export const TAILSCALE_DOWNLOAD = "https://tailscale.com/download";

export const PHASES = [
  "installingMap",
  "launchingGame",
  "launchInEpic",
  "startingBakkes",
  "waitingBakkes",
  "commandSent",
  "hostInRocketPlugin",
  "manual",
  "failed",
] as const;
export const phaseSchema = z.enum(PHASES);
export type Phase = z.infer<typeof phaseSchema>;

export const multiSessionSchema = z.object({
  id: z.string(),
  role: z.enum(["host", "join"]),
  phase: phaseSchema,
  mapId: z.string().nullable(),
  mapName: z.string(),
  target: z.string(),
  address: z.string().nullable(),
  inviteCode: z.string().nullable(),
  command: z.string().nullable(),
  mapMismatch: z.boolean(),
  message: z.string().nullable(),
  startedAt: z.string(),
});
export type MultiSession = z.infer<typeof multiSessionSchema>;

export const multiSessionEventSchema = multiSessionSchema.nullable();

const choiceSchema = z.object({ display: z.string(), internal: z.string() });
export const rpCatalogSchema = z.object({
  gameModes: z.array(choiceSchema),
  mutators: z.array(z.object({ category: z.string(), label: z.string(), values: z.array(choiceSchema) })),
  presets: z.array(z.string()),
});
export type RpCatalog = z.infer<typeof rpCatalogSchema>;

/** Mirrors `multi::rp_options::MatchOptions`. */
export interface MatchOptions {
  /** Start the match from Rocket Plugin's window (no command sent). */
  manual: boolean;
  gameMode: string | null;
  players: number | null;
  preset: string | null;
  /** Category → internal value. */
  mutators: Record<string, string>;
}

export const matchOptionsSchema = z.object({
  manual: z.boolean(),
  gameMode: z.string().nullable(),
  players: z.number().nullable(),
  preset: z.string().nullable(),
  mutators: z.record(z.string(), z.string()),
});

/** Saved host setup (`multi::presets::HostPreset`). */
export const hostPresetSchema = z.object({
  id: z.string(),
  name: z.string(),
  mapId: z.string(),
  mapName: z.string(),
  target: z.string(),
  options: matchOptionsSchema,
  savedAt: z.string(),
});
export type HostPreset = z.infer<typeof hostPresetSchema>;

export interface HostPresetInput {
  /** Set = overwrite that preset. */
  id: string | null;
  name: string;
  mapId: string;
  target: string;
  options: MatchOptions;
}

export const DEFAULT_MATCH_OPTIONS: MatchOptions = {
  manual: false,
  gameMode: null,
  players: null,
  preset: null,
  mutators: {},
};

export const multiApi = {
  rpOptions: () => call("multi_rp_options", {}, rpCatalogSchema),
  presetsList: () => call("multi_presets_list", {}, z.array(hostPresetSchema)),
  presetsSave: (preset: HostPresetInput) => call("multi_presets_save", { preset }, hostPresetSchema),
  presetsDelete: (id: string) => callVoid("multi_presets_delete", { id }),
  status: () => call("multi_status", {}, multiStatusSchema),
  bakkesSetup: () => call("multi_bakkes_setup", {}, bakkesSetupSchema),
  bakkesUndo: () => call("multi_bakkes_undo", {}, bakkesSetupSchema),
  installRocketPlugin: () => callVoid("multi_install_rocket_plugin"),
  host: (mapId: string, target: string, options: MatchOptions) =>
    call("multi_host", { mapId, target, options }, multiSessionSchema),
  check: (code: string) => call("multi_check", { code }, inviteCheckSchema),
  join: (code: string) => call("multi_join", { code }, multiSessionSchema),
  session: () => call("multi_session", {}, multiSessionSchema.nullable()),
  cancel: () => callVoid("multi_cancel"),
};
