import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { z } from "zod";
import { call } from "@/lib/ipc";

/** Owned by the engine slice (`upk::commands::keys_status`). */
export const keysStatusSchema = z.object({
  present: z.boolean(),
  count: z.number(),
  source: z.string().nullable().optional(),
  /** Game build the community list was last checked for. */
  checkedBuild: z.string().nullable().optional(),
  communityPage: z.string(),
});
export type KeysStatus = z.infer<typeof keysStatusSchema>;

/** Result of a download or an import (keys are merged, never removed). */
export const keysUpdateSchema = z.object({
  added: z.number(),
  total: z.number(),
  /** Catalog items readable now that were not before. */
  unlocked: z.number(),
  status: keysStatusSchema,
});
export type KeysUpdate = z.infer<typeof keysUpdateSchema>;

export function useKeysStatus() {
  return useQuery({ queryKey: ["keysStatus"], queryFn: () => call("keys_status", {}, keysStatusSchema) });
}

function useKeysMutation<T>(fn: (arg: T) => Promise<KeysUpdate>) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: (update) => {
      qc.setQueryData(["keysStatus"], update.status);
      // Catalog locks and decal status depend on the keys.
      void qc.invalidateQueries();
    },
  });
}

/** Merges the keys of a file chosen by the user. */
export function useImportKeys() {
  return useKeysMutation((path: string) => call("keys_import", { path }, keysUpdateSchema));
}

/** Downloads the community key list (validated, then merged). */
export function useDownloadKeys() {
  return useKeysMutation(() => call("keys_download", {}, keysUpdateSchema));
}

/** Hides the "update your keys" suggestion until the next game build. */
export function useDismissKeysUpdate() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => call("keys_dismiss_update", {}, keysStatusSchema),
    onSuccess: (status) => qc.setQueryData(["keysStatus"], status),
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
