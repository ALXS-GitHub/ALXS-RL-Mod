/**
 * Colour conversions shared by the colour picker, the palette editor and
 * the item colour options. Channels are 0..255, hue in degrees, saturation
 * and value in 0..1.
 */

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

export interface Hsv {
  h: number;
  s: number;
  v: number;
}

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

export function rgbToHex({ r, g, b }: Rgb): string {
  return `#${[r, g, b].map((v) => clamp(Math.round(v), 0, 255).toString(16).padStart(2, "0")).join("")}`;
}

/** `#rgb` or `#rrggbb` (with or without `#`), else `null`. */
export function hexToRgb(hex: string): Rgb | null {
  const raw = hex.trim().replace(/^#/, "");
  const full = /^[0-9a-f]{3}$/i.test(raw)
    ? raw
        .split("")
        .map((c) => c + c)
        .join("")
    : raw;
  if (!/^[0-9a-f]{6}$/i.test(full)) return null;
  const n = Number.parseInt(full, 16);
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
}

export function rgbToHsv({ r, g, b }: Rgb): Hsv {
  const [rn, gn, bn] = [r / 255, g / 255, b / 255];
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const d = max - min;
  let h = 0;
  if (d > 0) {
    if (max === rn) h = ((gn - bn) / d) % 6;
    else if (max === gn) h = (bn - rn) / d + 2;
    else h = (rn - gn) / d + 4;
  }
  return { h: (h * 60 + 360) % 360, s: max === 0 ? 0 : d / max, v: max };
}

export function hsvToRgb({ h, s, v }: Hsv): Rgb {
  const hue = ((h % 360) + 360) % 360;
  const c = v * s;
  const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
  const m = v - c;
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
  return { r: Math.round((r + m) * 255), g: Math.round((g + m) * 255), b: Math.round((b + m) * 255) };
}

export const hexToHsv = (hex: string): Hsv | null => {
  const rgb = hexToRgb(hex);
  return rgb ? rgbToHsv(rgb) : null;
};

export const hsvToHex = (hsv: Hsv): string => rgbToHex(hsvToRgb(hsv));

/** Normalised `#rrggbb` (lowercase) or `null`. */
export const normalizeHex = (hex: string): string | null => {
  const rgb = hexToRgb(hex);
  return rgb ? rgbToHex(rgb) : null;
};
