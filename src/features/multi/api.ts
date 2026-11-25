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

export const PHASES = [
  "downloadingMap",
  "installingMap",
  "launchingGame",
  "startingBakkes",
  "waitingBakkes",
  "commandSent",
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

export const multiApi = {
  status: () => call("multi_status", {}, multiStatusSchema),
  bakkesSetup: () => call("multi_bakkes_setup", {}, bakkesSetupSchema),
  bakkesUndo: () => call("multi_bakkes_undo", {}, bakkesSetupSchema),
  host: (mapId: string) => call("multi_host", { mapId }, multiSessionSchema),
  decode: (code: string) => call("multi_decode", { code }, inviteSchema),
  join: (code: string) => call("multi_join", { code }, multiSessionSchema),
  session: () => call("multi_session", {}, multiSessionSchema.nullable()),
  cancel: () => callVoid("multi_cancel"),
};
