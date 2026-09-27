import { z } from "zod";
import { call, callVoid } from "@/lib/ipc";

// Mirrors src-tauri/src/palette/{store,engine}.rs

export const rgbSchema = z.object({
  r: z.number().int().min(0).max(255),
  g: z.number().int().min(0).max(255),
  b: z.number().int().min(0).max(255),
});
export type Rgb = z.infer<typeof rgbSchema>;

export const paletteSchema = z.object({
  id: z.string(),
  name: z.string(),
  primaryBlue: z.array(rgbSchema),
  primaryOrange: z.array(rgbSchema),
  accent: z.array(rgbSchema),
  createdAt: z.string(),
  updatedAt: z.string(),
});
export type Palette = z.infer<typeof paletteSchema>;

/** What the editor sends: `id` empty for a new palette. */
export type PaletteDraft = Pick<Palette, "id" | "name" | "primaryBlue" | "primaryOrange" | "accent">;

export const paletteStatusSchema = z.object({
  engine: z.enum(["ready", "gameMissing", "unsupported"]),
  reason: z.string().nullable(),
  active: z
    .object({
      paletteId: z.string(),
      paletteName: z.string(),
      appliedAt: z.string(),
      stale: z.boolean(),
    })
    .nullable(),
  rows: z.number().int(),
  primarySlots: z.number().int(),
  accentSlots: z.number().int(),
});
export type PaletteStatus = z.infer<typeof paletteStatusSchema>;

export const STOCK_ID = "stock";

export const paletteApi = {
  status: () => call("palette_status", {}, paletteStatusSchema),
  stock: () => call("palette_stock", {}, paletteSchema),
  list: () => call("palette_list", {}, z.array(paletteSchema)),
  save: (palette: PaletteDraft) => call("palette_save", { palette }, paletteSchema),
  remove: (id: string) => callVoid("palette_delete", { id }),
  apply: (id: string) => call("palette_apply", { id }, paletteStatusSchema),
  restore: () => call("palette_restore", {}, paletteStatusSchema),
};
