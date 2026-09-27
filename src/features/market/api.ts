import { z } from "zod";
import { call } from "@/lib/ipc";

// Mirrors src-tauri/src/market/model.rs

export const MARKET_SOURCES = ["alphaConsole", "rlDesigner"] as const;
export type MarketSource = (typeof MARKET_SOURCES)[number];

export const MARKET_KINDS = ["decal", "ball"] as const;
export type MarketKind = (typeof MARKET_KINDS)[number];

export const MARKET_SORTS = ["downloads", "likes", "newest", "updated", "trending"] as const;
export type MarketSort = (typeof MARKET_SORTS)[number];

export const marketItemSchema = z.object({
  source: z.enum(MARKET_SOURCES),
  id: z.string(),
  kind: z.enum(MARKET_KINDS),
  title: z.string(),
  author: z.string().nullable(),
  description: z.string().nullable(),
  thumbnail: z.string().nullable(),
  pageUrl: z.string().nullable(),
  downloads: z.number().nullable(),
  likes: z.number().nullable(),
  bodies: z.array(z.string()),
  ready: z.boolean(),
  installed: z.boolean(),
});
export type MarketItem = z.infer<typeof marketItemSchema>;

export const marketPageSchema = z.object({
  items: z.array(marketItemSchema),
  total: z.number().nullable(),
  truncated: z.boolean(),
});
export type MarketPage = z.infer<typeof marketPageSchema>;

export const installReportSchema = z.object({
  pack: z.string(),
  variants: z.number(),
  converted: z.number(),
  failed: z.array(z.string()),
});
export type InstallReport = z.infer<typeof installReportSchema>;

export interface InstallArgs {
  item: MarketItem;
  convert: boolean;
}

export const marketApi = {
  browse: (source: MarketSource, kind: MarketKind, query: string, sort: MarketSort) =>
    call("market_browse", { source, kind, query, sort }, marketPageSchema),
  install: ({ item, convert }: InstallArgs) =>
    call(
      "market_install",
      { source: item.source, kind: item.kind, id: item.id, title: item.title, convert },
      installReportSchema,
    ),
};
