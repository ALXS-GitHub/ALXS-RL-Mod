import {
  Activity,
  Brush,
  Gamepad2,
  House,
  Layers,
  type LucideIcon,
  Map as MapIcon,
  Palette,
  Puzzle,
  Settings,
  Sparkles,
  Volleyball,
} from "lucide-react";

export type NavSection = "overview" | "game" | "garage" | "arena" | "stats" | "system";

export interface NavItem {
  to: string;
  /** Key in the `common` namespace (`nav.<key>`). */
  labelKey: string;
  icon: LucideIcon;
  section: NavSection;
  /** Command palette keywords (both languages). */
  keywords: string[];
  /** Maturity tag shown next to the label (`common:nav.stages.<stage>`). */
  stage?: "experimental";
  /** The page changes game files: the active mods panel opens by default. */
  modifiesGame?: boolean;
}

/** Single source of truth for the sidebar, the router and the command palette. */
export const NAV: readonly NavItem[] = [
  {
    to: "/",
    labelKey: "nav.home",
    icon: House,
    section: "overview",
    keywords: ["home", "accueil", "dashboard"],
  },
  {
    to: "/play",
    labelKey: "nav.play",
    icon: Gamepad2,
    section: "game",
    keywords: ["launch", "lancer", "jouer", "eac", "offline"],
  },
  {
    to: "/presets",
    labelKey: "nav.presets",
    icon: Layers,
    section: "garage",
    keywords: ["loadout", "preset", "share", "partage"],
    modifiesGame: true,
  },
  {
    to: "/items",
    labelKey: "nav.items",
    icon: Sparkles,
    section: "garage",
    keywords: ["swap", "wheels", "roues", "boost", "decal", "car", "voiture"],
    modifiesGame: true,
  },
  {
    to: "/palette",
    labelKey: "nav.palette",
    icon: Palette,
    section: "garage",
    keywords: ["colors", "couleurs", "paint"],
    modifiesGame: true,
  },
  {
    to: "/decals",
    labelKey: "nav.decals",
    icon: Brush,
    section: "garage",
    keywords: ["custom", "png", "alphaconsole", "sticker"],
    modifiesGame: true,
    stage: "experimental",
  },
  {
    to: "/ball",
    labelKey: "nav.ball",
    icon: Volleyball,
    section: "garage",
    keywords: ["ball", "balle", "texture", "alphaconsole"],
    modifiesGame: true,
    stage: "experimental",
  },
  {
    to: "/maps",
    labelKey: "nav.maps",
    icon: MapIcon,
    section: "arena",
    keywords: ["workshop", "map", "bakkesplugins", "lethamyr"],
    modifiesGame: true,
  },
  {
    to: "/tracker",
    labelKey: "nav.tracker",
    icon: Activity,
    section: "stats",
    keywords: ["tracker", "mmr", "overlay", "stats", "session", "rank", "rang"],
  },
  {
    to: "/extras",
    labelKey: "nav.extras",
    icon: Puzzle,
    section: "system",
    keywords: ["replays", "bakkesmod"],
  },
  {
    to: "/settings",
    labelKey: "nav.settings",
    icon: Settings,
    section: "system",
    keywords: ["settings", "réglages", "language", "langue"],
  },
];

/** Whether `pathname` is a page that changes game files. */
export function modifiesGame(pathname: string): boolean {
  return NAV.some((n) => n.modifiesGame && n.to !== "/" && pathname.startsWith(n.to));
}

export const NAV_SECTIONS: readonly NavSection[] = ["overview", "game", "garage", "arena", "stats", "system"];
