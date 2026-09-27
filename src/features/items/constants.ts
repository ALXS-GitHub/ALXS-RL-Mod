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
export const PAINTS: readonly { id: number; key: string; hex: string }[] = [
  { id: 0, key: "none", hex: "transparent" },
  { id: 1, key: "crimson", hex: "#d11a1a" },
  { id: 2, key: "lime", hex: "#8cf018" },
  { id: 3, key: "black", hex: "#141414" },
  { id: 4, key: "orange", hex: "#ff8a1a" },
  { id: 5, key: "skyBlue", hex: "#5fd3ff" },
  { id: 6, key: "cobalt", hex: "#3150d6" },
  { id: 7, key: "saffron", hex: "#f2d11a" },
  { id: 8, key: "grey", hex: "#8c8c8c" },
  { id: 9, key: "pink", hex: "#ff6fb5" },
  { id: 10, key: "forestGreen", hex: "#1f8a3a" },
  { id: 11, key: "purple", hex: "#8a3ce0" },
  { id: 12, key: "titaniumWhite", hex: "#f4f4f4" },
];

export function itemLabel(item: Pick<CatalogItem, "labelFr" | "labelEn">): string {
  return i18n.language === "fr" ? item.labelFr : item.labelEn;
}

/** Accent-insensitive, case-insensitive search key. */
export function searchKey(s: string): string {
  return s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}
