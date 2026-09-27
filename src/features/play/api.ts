import { z } from "zod";
import { call } from "@/lib/ipc";

// Mirrors src-tauri/src/extras/launch.rs

export const launchResultSchema = z.object({
  withEac: z.boolean(),
  method: z.enum(["direct", "epicLauncher", "steamLauncher"]),
  pid: z.number().nullable(),
});
export type LaunchResult = z.infer<typeof launchResultSchema>;

export const playApi = {
  launch: (withEac: boolean) => call("launch_game", { withEac }, launchResultSchema),
};
