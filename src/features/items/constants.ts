import {
  Brush,
  Car,
  CircleDot,
  Flag,
  type LucideIcon,
  PaintBucket,
  Rocket,
  Sparkle,
  Tornado,
  TreePine,
} from "lucide-react";
import i18n from "@/lib/i18n";
import type { CatalogItem, Slot } from "./api";

export const SLOT_ICONS: Record<Slot, LucideIcon> = {
  body: Car,
  decal: Brush,
  wheels: CircleDot,
  boost: Rocket,
  topper: TreePine,
  antenna: Flag,
  goalExplosion: Sparkle,
  trail: Tornado,
  paintFinish: PaintBucket,
};

/** In-game paint ids (0 = none) with display colours. */

export function itemLabel(item: Pick<CatalogItem, "labelFr" | "labelEn">): string {
  return i18n.language === "fr" ? item.labelFr : item.labelEn;
}

/** Accent-insensitive, case-insensitive search key. */
export function searchKey(s: string): string {
  return s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}
