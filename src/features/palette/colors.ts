import type { AccentTriplet } from "@/stores/fx";
import type { Rgb } from "./api";

/** RL picker geometry: 7 shade rows × N hue columns, row-major. */
export const PICKER_ROWS = 7;
export const PRIMARY_COLUMNS = 10;
export const ACCENT_COLUMNS = 15;

export const toHex = ({ r, g, b }: Rgb) =>
  `#${[r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")}`;

export function fromHex(hex: string): Rgb | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m?.[1]) return null;
  const n = Number.parseInt(m[1], 16);
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
}

export function toHsl({ r, g, b }: Rgb): [number, number, number] {
  const [rn, gn, bn] = [r / 255, g / 255, b / 255];
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h: number;
  if (max === rn) h = (gn - bn) / d + (gn < bn ? 6 : 0);
  else if (max === gn) h = (bn - rn) / d + 2;
  else h = (rn - gn) / d + 4;
  return [h * 60, s, l];
}

export function fromHsl(h: number, s: number, l: number): Rgb {
  const hue = ((h % 360) + 360) % 360;
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
  const m = l - c / 2;
  const [r, g, b] =
    hue < 60
      ? [c, x, 0]
      : hue < 120
        ? [x, c, 0]
        : hue < 180
          ? [0, c, x]
          : hue < 240
            ? [0, x, c]
            : hue < 300
              ? [x, 0, c]
              : [c, 0, x];
  const to = (v: number) => Math.round(Math.min(1, Math.max(0, v + m)) * 255);
  return { r: to(r), g: to(g), b: to(b) };
}

export const mix = (a: Rgb, b: Rgb, t: number): Rgb => ({
  r: Math.round(a.r + (b.r - a.r) * t),
  g: Math.round(a.g + (b.g - a.g) * t),
  b: Math.round(a.b + (b.b - a.b) * t),
});

/** Fills `count` slots, padding with `fallback` (stock) colours. */
export function normalize(colors: readonly Rgb[], fallback: readonly Rgb[], count: number): Rgb[] {
  return Array.from({ length: count }, (_, i) => colors[i] ?? fallback[i] ?? { r: 0, g: 0, b: 0 });
}

/** Most saturated, mid-lightness colour of a set — used to tint the aurora. */
export function vivid(colors: readonly Rgb[], fallback: string): string {
  let best: Rgb | undefined;
  let score = -1;
  for (const c of colors) {
    const [, s, l] = toHsl(c);
    const v = s * (1 - Math.abs(l - 0.5) * 1.6);
    if (v > score) {
      score = v;
      best = c;
    }
  }
  return best ? toHex(best) : fallback;
}

export function auroraFrom(
  blue: readonly Rgb[],
  orange: readonly Rgb[],
  accent: readonly Rgb[],
): AccentTriplet {
  return [vivid(accent, "#7c6cff"), vivid(blue, "#2f7bff"), vivid(orange, "#ff7a1a")];
}

// ── Generators (whole grid) ───────────────────────────────────────────────

/**
 * RL-like layout: each column is one hue, each row one shade from light to
 * dark. `hues` gives the column hues in degrees.
 */
export function shadeGrid(hues: readonly number[], rows = PICKER_ROWS, saturation = 0.85): Rgb[] {
  const out: Rgb[] = [];
  for (let r = 0; r < rows; r++) {
    const l = 0.82 - (r / Math.max(1, rows - 1)) * 0.62;
    for (const h of hues) out.push(fromHsl(h, saturation, l));
  }
  return out;
}

export function rainbowGrid(columns: number, rows = PICKER_ROWS, offset = 0): Rgb[] {
  return shadeGrid(
    Array.from({ length: columns }, (_, i) => offset + (i * 360) / columns),
    rows,
  );
}

/** Two-colour gradient laid out column by column. */
export function gradientGrid(from: Rgb, to: Rgb, columns: number, rows = PICKER_ROWS): Rgb[] {
  const out: Rgb[] = [];
  for (let r = 0; r < rows; r++) {
    const shade = r / Math.max(1, rows - 1);
    for (let c = 0; c < columns; c++) {
      const base = mix(from, to, c / Math.max(1, columns - 1));
      out.push(
        mix(mix(base, { r: 255, g: 255, b: 255 }, 0.35 * (1 - shade)), { r: 0, g: 0, b: 0 }, 0.55 * shade),
      );
    }
  }
  return out;
}

export function hueShift(colors: readonly Rgb[], degrees: number): Rgb[] {
  return colors.map((c) => {
    const [h, s, l] = toHsl(c);
    return fromHsl(h + degrees, s, l);
  });
}

/** Random analogous/complementary harmony around a random base hue. */
export function harmonyGrid(columns: number, rows = PICKER_ROWS, seed = Math.random()): Rgb[] {
  const base = seed * 360;
  const spread = 30 + seed * 90;
  const hues = Array.from(
    { length: columns },
    (_, i) => base + (i - columns / 2) * (spread / columns) + (i % 2 ? 180 : 0),
  );
  return shadeGrid(hues, rows, 0.7 + seed * 0.25);
}
