/**
 * Items IPC: catalog (local), swaps, integrity. Schemas mirror
 * `src-tauri/src/{catalog,swap,integrity}`.
 */
import { z } from "zod";
import { integrityReportSchema } from "@/lib/integrity";
import { call, callVoid } from "@/lib/ipc";

export const SLOTS = [
  "body",
  "decal",
  "wheels",
  "boost",
  "topper",
  "antenna",
  "goalExplosion",
  "trail",
  "paintFinish",
] as const;
export const slotSchema = z.enum(SLOTS);
export type Slot = z.infer<typeof slotSchema>;

export const catalogItemSchema = z.object({
  id: z.number(),
  slot: slotSchema,
  asset: z.string(),
  package: z.string(),
  thumbnailPackage: z.string().nullable(),
  labelFr: z.string(),
  labelEn: z.string(),
  bodyId: z.number().nullable(),
  sharedPackage: z.boolean(),
  locked: z.boolean(),
});
export type CatalogItem = z.infer<typeof catalogItemSchema>;

export const catalogSnapshotSchema = z.object({
  items: z.array(catalogItemSchema),
  counts: z.array(z.object({ slot: slotSchema, count: z.number() })),
  unresolved: z.number(),
  fromGame: z.boolean(),
  build: z.string(),
  generatedAt: z.string(),
});
export type CatalogSnapshot = z.infer<typeof catalogSnapshotSchema>;

export const swapRequestSchema = z.object({
  slot: slotSchema,
  ownedId: z.number(),
  wantedId: z.number(),
  paint: z.number().nullable().optional(),
  /** Experimental recolour: hue in degrees (0..359). */
  tint: z.number().nullable().optional(),
});
export type SwapRequest = z.infer<typeof swapRequestSchema>;

export const activeSwapSchema = z.object({
  id: z.string(),
  request: swapRequestSchema,
  ownedLabel: z.string(),
  wantedLabel: z.string(),
  targetPackage: z.string(),
  sourcePackage: z.string(),
  painted: z.boolean(),
  appliedAt: z.string(),
  build: z.string(),
});
export type ActiveSwap = z.infer<typeof activeSwapSchema>;

export const swapEventSchema = z.object({
  at: z.string(),
  kind: z.enum(["applied", "restored", "reapplied", "dropped"]),
  swapId: z.string(),
  slot: slotSchema,
  ownedLabel: z.string(),
  wantedLabel: z.string(),
  detail: z.string().nullable().optional(),
});
export type SwapEvent = z.infer<typeof swapEventSchema>;

export const itemsApi = {
  catalog: () => call("catalog_get", {}, catalogSnapshotSchema),
  refreshCatalog: () => call("catalog_refresh", {}, catalogSnapshotSchema),
  swaps: () => call("swap_list", {}, z.array(activeSwapSchema)),
  apply: (req: SwapRequest) => call("swap_apply", { req }, activeSwapSchema),
  restore: (id: string) => callVoid("swap_restore", { id }),
  restoreAll: () => call("swap_restore_all", {}, z.number()),
  history: () => call("swap_history", {}, z.array(swapEventSchema)),
  integrityCheck: () => call("integrity_check", {}, integrityReportSchema),
  integrityReapply: () => call("integrity_reapply", {}, integrityReportSchema),
  /** Absolute path of a cached PNG thumbnail (engine slice), or null. */
  thumbnail: (pkg: string) => call("thumbnail_get", { package: pkg }, z.string().nullable()),
};
