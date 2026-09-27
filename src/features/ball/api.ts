import { z } from "zod";
import { alphaConsoleSourceSchema, importReportSchema } from "@/features/decals/api";
import { call } from "@/lib/ipc";

// Mirrors src-tauri/src/ball/{library,engine,commands}.rs

export const ballPackSchema = z.object({
  id: z.string(),
  packName: z.string(),
  /** Ball the pack was made for (`Default` = standard ball). */
  ball: z.string(),
  displayName: z.string(),
  group: z.string(),
  imagePath: z.string().nullable(),
  previewPath: z.string().nullable(),
  templatePath: z.string(),
  supported: z.boolean(),
});
export type BallPack = z.infer<typeof ballPackSchema>;

export const ballStatusSchema = z.object({
  active: z
    .object({
      packId: z.string(),
      displayName: z.string(),
      appliedAt: z.string(),
      stale: z.boolean(),
      previewPath: z.string().nullable(),
    })
    .nullable(),
  keysPresent: z.boolean(),
  gameFound: z.boolean(),
  alphaConsole: alphaConsoleSourceSchema.nullable(),
});
export type BallStatus = z.infer<typeof ballStatusSchema>;

export const ballApi = {
  library: () => call("ball_library", {}, z.array(ballPackSchema)),
  status: () => call("ball_status", {}, ballStatusSchema),
  apply: (packId: string) => call("ball_apply", { packId }, ballStatusSchema),
  remove: () => call("ball_remove", {}, ballStatusSchema),
  importAlphaConsole: () => call("ball_import_alphaconsole", {}, importReportSchema),
};
