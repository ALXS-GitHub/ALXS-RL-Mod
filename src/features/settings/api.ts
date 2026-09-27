import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { z } from "zod";
import { call } from "@/lib/ipc";

/** Owned by the engine slice (`upk::commands::keys_status`). */
export const keysStatusSchema = z.object({
  present: z.boolean(),
  count: z.number(),
  source: z.string().nullable().optional(),
});
export type KeysStatus = z.infer<typeof keysStatusSchema>;

export function useKeysStatus() {
  return useQuery({ queryKey: ["keysStatus"], queryFn: () => call("keys_status", {}, keysStatusSchema) });
}

/** Imports a keys.txt chosen by the user (copied to the app data folder). */
export function useImportKeys() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (path: string) => call("keys_import", { path }, keysStatusSchema),
    onSuccess: (status) => {
      qc.setQueryData(["keysStatus"], status);
      // Catalog locks and decal status depend on the keys.
      void qc.invalidateQueries();
    },
  });
}

export function useExportDiagnostics() {
  return useMutation({ mutationFn: () => call("diagnostics_export", {}, z.string()) });
}

export const cleanReportSchema = z.object({
  swaps: z.number(),
  palette: z.boolean(),
  decal: z.boolean(),
  ball: z.boolean().default(false),
  map: z.boolean(),
  leftovers: z.number(),
  failures: z.array(z.object({ feature: z.string(), message: z.string() })),
});
export type CleanReport = z.infer<typeof cleanReportSchema>;

/** Removes every modification the app made to the game (swaps, palette, decal, map). */
export function useRestoreStock() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => call("integrity_restore_stock", {}, cleanReportSchema),
    // Every feature's state changed: refetch everything.
    onSettled: () => void qc.invalidateQueries(),
  });
}
