import { z } from "zod";
import { call } from "@/lib/ipc";

// Mirrors src-tauri/src/extras/{replays,bakkes}.rs

export const replayHeaderSchema = z.object({
  name: z.string().nullable(),
  map: z.string().nullable(),
  date: z.string().nullable(),
  teamSize: z.number().nullable(),
  blueScore: z.number().nullable(),
  orangeScore: z.number().nullable(),
  durationSeconds: z.number().nullable(),
  matchType: z.string().nullable(),
  playerName: z.string().nullable(),
});
export type ReplayHeader = z.infer<typeof replayHeaderSchema>;

export const replayFileSchema = z.object({
  fileName: z.string(),
  path: z.string(),
  sizeBytes: z.number(),
  modifiedAt: z.string().nullable(),
  header: replayHeaderSchema.nullable(),
});
export type ReplayFile = z.infer<typeof replayFileSchema>;

export const replayListSchema = z.object({
  folders: z.array(z.string()),
  replays: z.array(replayFileSchema),
});

export const bakkesStatusSchema = z.object({
  installed: z.boolean(),
  dataDir: z.string().nullable(),
  injectorPath: z.string().nullable(),
  plugins: z.array(z.object({ name: z.string(), sizeBytes: z.number() })),
  alphaConsoleData: z.boolean(),
  workshopDir: z.string().nullable(),
});
export type BakkesStatus = z.infer<typeof bakkesStatusSchema>;

export const extrasApi = {
  replays: () => call("replays_list", {}, replayListSchema),
  bakkes: () => call("bakkesmod_status", {}, bakkesStatusSchema),
};
