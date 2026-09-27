/** Presets IPC. Mirrors `src-tauri/src/presets`. */
import { z } from "zod";
import { type Slot, swapRequestSchema } from "@/features/items/api";
import { call, callVoid } from "@/lib/ipc";

export const presetSchema = z.object({
  id: z.string(),
  name: z.string(),
  createdAt: z.string(),
  updatedAt: z.string(),
  swaps: z.array(swapRequestSchema),
  /** Owned by the palette feature — opaque here, only displayed. */
  palette: z.unknown().nullable().optional(),
  /** Owned by the decals feature — opaque here, only displayed. */
  decal: z.unknown().nullable().optional(),
  mapId: z.string().nullable().optional(),
});
export type Preset = z.infer<typeof presetSchema>;

export const presetPartSchema = z.enum(["swaps", "palette", "decal", "map"]);
export type PresetPart = z.infer<typeof presetPartSchema>;

export const applyReportSchema = z.object({
  presetId: z.string(),
  ok: z.boolean(),
  parts: z.array(
    z.object({ part: presetPartSchema, ok: z.boolean(), applied: z.number(), errors: z.array(z.string()) }),
  ),
});
export type PresetApplyReport = z.infer<typeof applyReportSchema>;

export const activePresetSchema = z.object({
  presetId: z.string(),
  name: z.string(),
  appliedAt: z.string(),
  /** Something changed since the preset was applied. */
  modified: z.boolean(),
});
export type ActivePreset = z.infer<typeof activePresetSchema>;

export const presetsApi = {
  active: () => call("presets_active", {}, activePresetSchema.nullable()),
  list: () => call("presets_list", {}, z.array(presetSchema)),
  save: (preset: Preset) => call("presets_save", { preset }, presetSchema),
  remove: (id: string) => callVoid("presets_delete", { id }),
  apply: (id: string) => call("presets_apply", { id }, applyReportSchema),
  capture: (name: string) => call("presets_capture", { name }, presetSchema),
  updateFromCurrent: (id: string) => call("presets_update_from_current", { id }, presetSchema),
  exportCode: (id: string) => call("presets_export_code", { id }, z.string()),
  importCode: (code: string) => call("presets_import_code", { code }, presetSchema),
  random: (slots: Slot[]) => call("presets_random", { slots }, presetSchema),
};

/** Best-effort display name of an opaque palette / decal payload. */
export function payloadName(payload: unknown): string | null {
  if (payload && typeof payload === "object" && "name" in payload) {
    const name = (payload as { name: unknown }).name;
    if (typeof name === "string" && name.trim()) return name;
  }
  return null;
}

/** Best-effort colours of an opaque palette payload (hex strings found at the top level). */
export function paletteSwatches(payload: unknown, max = 6): string[] {
  const out: string[] = [];
  const visit = (value: unknown, depth: number) => {
    if (out.length >= max || depth > 3) return;
    if (typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value)) out.push(value);
    else if (Array.isArray(value)) for (const v of value) visit(v, depth + 1);
    else if (value && typeof value === "object") {
      const o = value as Record<string, unknown>;
      if (typeof o.r === "number" && typeof o.g === "number" && typeof o.b === "number") {
        const to = (n: number) =>
          Math.round(n <= 1 ? n * 255 : n)
            .toString(16)
            .padStart(2, "0");
        out.push(`#${to(o.r)}${to(o.g)}${to(o.b)}`);
      } else for (const v of Object.values(o)) visit(v, depth + 1);
    }
  };
  visit(payload, 0);
  return out;
}
