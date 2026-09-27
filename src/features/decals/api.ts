import { z } from "zod";
import { call } from "@/lib/ipc";

// Mirrors src-tauri/src/decals/{library,targets,engine}.rs

const decalTextureSchema = z.object({
  role: z.string(),
  fileName: z.string(),
  path: z.string(),
  exists: z.boolean(),
});

export const decalPackSchema = z.object({
  id: z.string(),
  libraryRoot: z.string(),
  packName: z.string(),
  bodyFolder: z.string(),
  displayName: z.string(),
  group: z.string(),
  bodyId: z.number().int(),
  bodyName: z.string().nullable(),
  skinId: z.number().int(),
  body: z.array(decalTextureSchema),
  chassis: z.array(decalTextureSchema),
  templatePath: z.string(),
  previewPath: z.string().nullable(),
  maskPreviewPath: z.string().nullable(),
  maskPath: z.string().nullable(),
  diffusePath: z.string().nullable(),
  supported: z.boolean(),
  /** Art (`1_Diffuse_Skin`) + zone mask (`2_Diffuse_Skin_Mask`): colours kept outside the zones. */
  hybrid: z.boolean().default(false),
  /** Universal pack: `1_Diffuse_Skin` art + `TrimSheet` logos, on every car. */
  universal: z.boolean().default(false),
});
export type DecalPack = z.infer<typeof decalPackSchema>;

export const targetSlotSchema = z.object({
  key: z.string(),
  label: z.string(),
  bodyId: z.number().int(),
  bodyName: z.string(),
  fileName: z.string(),
});
export type TargetSlot = z.infer<typeof targetSlotSchema>;

export const libraryRootSchema = z.object({
  path: z.string(),
  exists: z.boolean(),
  packCount: z.number().int(),
  isDefault: z.boolean(),
});
export type LibraryRoot = z.infer<typeof libraryRootSchema>;

export const alphaConsoleSourceSchema = z.object({
  path: z.string(),
  packCount: z.number().int(),
  notImported: z.number().int(),
});

export const importReportSchema = z.object({
  imported: z.number().int(),
  already: z.number().int(),
  converted: z.number().int(),
  failed: z.array(z.string()),
});
export type ImportReport = z.infer<typeof importReportSchema>;

export const decalStatusSchema = z.object({
  active: z
    .object({
      packId: z.string(),
      displayName: z.string(),
      bodyName: z.string(),
      target: targetSlotSchema.nullable(),
      appliedAt: z.string(),
      stale: z.boolean(),
      previewPath: z.string().nullable().default(null),
    })
    .nullable(),
  targets: z.array(targetSlotSchema),
  libraryRoots: z.array(libraryRootSchema),
  keysPresent: z.boolean(),
  gameFound: z.boolean(),
  alphaConsole: alphaConsoleSourceSchema.nullable().default(null),
});
export type DecalStatus = z.infer<typeof decalStatusSchema>;

export interface DecalApplyRequest {
  packId: string;
  target?: string | null;
}

export const decalsApi = {
  library: () => call("decals_library", {}, z.array(decalPackSchema)),
  setFolders: (folders: string[]) =>
    call("decals_library_folders_set", { folders }, z.array(decalPackSchema)),
  status: () => call("decals_status", {}, decalStatusSchema),
  apply: (req: DecalApplyRequest) => call("decals_apply", { req }, decalStatusSchema),
  remove: () => call("decals_remove", {}, decalStatusSchema),
  importAlphaConsole: (convert: boolean) =>
    call("decals_import_alphaconsole", { convert }, importReportSchema),
};
