/** Log viewer IPC. Mirrors `src-tauri/src/base/logs.rs`. */
import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { z } from "zod";
import { call } from "@/lib/ipc";

export const logSourceSchema = z.enum(["app", "game"]);
export type LogSource = z.infer<typeof logSourceSchema>;

export const logLevelSchema = z.enum(["error", "warn", "info", "debug"]);
export type LogLevel = z.infer<typeof logLevelSchema>;

export const logLineSchema = z.object({
  time: z.string().nullable(),
  level: logLevelSchema,
  target: z.string().nullable(),
  message: z.string(),
});
export type LogLine = z.infer<typeof logLineSchema>;

export const logViewSchema = z.object({
  source: logSourceSchema,
  file: z.string().nullable(),
  files: z.array(z.object({ name: z.string(), size: z.number(), modified: z.string().nullable() })),
  lines: z.array(logLineSchema),
  truncated: z.boolean(),
  folder: z.string().nullable(),
});
export type LogView = z.infer<typeof logViewSchema>;

export const logsKeys = {
  all: ["logs"] as const,
  view: (source: LogSource, file: string | null) => ["logs", source, file] as const,
};

export function useLogs(source: LogSource, file: string | null, follow: boolean) {
  return useQuery({
    queryKey: logsKeys.view(source, file),
    queryFn: () => call("logs_read", { source, file }, logViewSchema),
    placeholderData: keepPreviousData,
    refetchInterval: follow ? 3000 : false,
  });
}

export function useCleanLogs() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => call("logs_clean", {}, z.number()),
    onSettled: () => void qc.invalidateQueries({ queryKey: logsKeys.all }),
  });
}
