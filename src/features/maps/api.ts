import { z } from "zod";
import { call, callVoid } from "@/lib/ipc";

// Mirrors src-tauri/src/maps/model.rs

/** Labs arenas a custom map can replace (first = community default). */
export const LABS_TARGETS = [
  "Labs_Underpass_P.upk",
  "Labs_Utopia_P.upk",
  "Labs_Cosmic_V4_P.upk",
  "Labs_Cosmic_P.upk",
  "Labs_Octagon_02_P.upk",
  "Labs_Octagon_P.upk",
  "Labs_Galleon_P.upk",
  "Labs_Holyfield_P.upk",
  "Labs_Basin_P.upk",
  "Labs_CirclePillars_P.upk",
  "Labs_Corridor_P.upk",
  "Labs_DoubleGoal_V2_P.upk",
  "Labs_DoubleGoal_P.upk",
  "Labs_PillarGlass_P.upk",
  "Labs_PillarHeat_P.upk",
  "Labs_PillarWings_P.upk",
] as const;
export const DEFAULT_TARGET = LABS_TARGETS[0];

/** `Labs_Underpass_P.upk` → `Underpass`. */
export function targetLabel(target: string): string {
  return target
    .replace(/^Labs_/, "")
    .replace(/_P\.upk$/i, "")
    .replace(/_/g, " ");
}

export const remoteSourceSchema = z.enum(["bakkesPlugins", "lethamyr"]);
export type RemoteSource = z.infer<typeof remoteSourceSchema>;

/** Browse order (bakkesplugins only). */
export const MAP_SORTS = ["downloads", "newest", "rating", "views"] as const;
export const mapSortSchema = z.enum(MAP_SORTS);
export type MapSort = z.infer<typeof mapSortSchema>;

export const mapOriginSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("imported") }),
  z.object({ kind: z.literal("bakkesMod"), folderName: z.string() }),
  z.object({
    kind: z.literal("remote"),
    source: remoteSourceSchema,
    remoteId: z.string(),
    version: z.string().nullable(),
  }),
  z.object({ kind: z.literal("other"), url: z.string().nullable() }),
]);
export type MapOrigin = z.infer<typeof mapOriginSchema>;

export const mapEntrySchema = z.object({
  id: z.string(),
  name: z.string(),
  author: z.string().nullable(),
  description: z.string().nullable(),
  tags: z.array(z.string()),
  origin: mapOriginSchema,
  fileName: z.string(),
  companions: z.array(z.string()),
  previewFile: z.string().nullable(),
  previewUrl: z.string().nullable(),
  sizeBytes: z.number(),
  addedAt: z.string(),
  favorite: z.boolean(),
  preferredTarget: z.string().nullable(),
  lastPlayedAt: z.string().nullable(),
  folder: z.string(),
});
export type MapEntry = z.infer<typeof mapEntrySchema>;

export const mapSessionSchema = z.object({
  mapId: z.string(),
  mapName: z.string(),
  target: z.string(),
  activatedAt: z.string(),
  offline: z.boolean(),
});
export type MapSession = z.infer<typeof mapSessionSchema>;

export const remoteMapSchema = z.object({
  source: remoteSourceSchema,
  remoteId: z.string(),
  name: z.string(),
  author: z.string().nullable(),
  description: z.string().nullable(),
  previewUrl: z.string().nullable(),
  sizeBytes: z.number().nullable(),
  tags: z.array(z.string()),
  updatedAt: z.string().nullable(),
  downloadCount: z.number().nullish(),
  averageRating: z.number().nullish(),
  ratingCount: z.number().nullish(),
  latestVersionString: z.string().nullish(),
  pageUrl: z.string(),
  downloadable: z.boolean(),
  installedMapId: z.string().nullable(),
});
export type RemoteMap = z.infer<typeof remoteMapSchema>;

export const browseResultSchema = z.object({
  source: remoteSourceSchema,
  page: z.number(),
  totalPages: z.number().nullable(),
  hasNext: z.boolean(),
  items: z.array(remoteMapSchema),
});
export type BrowseResult = z.infer<typeof browseResultSchema>;

export const downloadProgressSchema = z.object({
  source: remoteSourceSchema,
  remoteId: z.string(),
  phase: z.enum(["download", "verify", "extract", "done", "failed"]),
  downloaded: z.number(),
  total: z.number().nullable(),
});
export type DownloadProgress = z.infer<typeof downloadProgressSchema>;

export interface MapPatch {
  name?: string;
  favorite?: boolean;
  tags?: string[];
  /** `null` clears the preference. */
  preferredTarget?: string | null;
}

export const mapsApi = {
  list: () => call("maps_list", {}, z.array(mapEntrySchema)),
  import: (paths: string[]) => call("maps_import", { paths }, z.array(mapEntrySchema)),
  remove: (id: string) => callVoid("maps_delete", { id }),
  update: (id: string, patch: MapPatch) => call("maps_update", { id, patch }, mapEntrySchema),
  browse: (source: RemoteSource, query: string, page: number, sort: MapSort) =>
    call("maps_browse", { source, query, page, sort }, browseResultSchema),
  download: (source: RemoteSource, remoteId: string) =>
    call("maps_download", { source, remoteId }, mapEntrySchema),
  activate: (id: string, target: string | null) => call("maps_activate", { id, target }, mapSessionSchema),
  deactivate: () => callVoid("maps_deactivate"),
  session: () => call("maps_session", {}, mapSessionSchema.nullable()),
  playOffline: (id: string) => call("maps_play_offline", { id }, mapSessionSchema),
};
