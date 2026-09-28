import { describe, expect, it } from "vitest";
import { hexToHsv, hexToRgb, hsvToHex, normalizeHex, rgbToHex, rgbToHsv } from "./color";

describe("color", () => {
  it("parses short and long hex, with or without #", () => {
    expect(hexToRgb("#ff8000")).toEqual({ r: 255, g: 128, b: 0 });
    expect(hexToRgb("0f0")).toEqual({ r: 0, g: 255, b: 0 });
    expect(hexToRgb("#12345")).toBeNull();
    expect(hexToRgb("zzzzzz")).toBeNull();
    expect(normalizeHex("ABC")).toBe("#aabbcc");
  });

  it("round-trips through HSV", () => {
    for (const hex of ["#ff0000", "#00ff00", "#0000ff", "#e3a23b", "#2b1245", "#ffffff", "#000000"]) {
      const hsv = hexToHsv(hex);
      expect(hsv).not.toBeNull();
      if (hsv) expect(hsvToHex(hsv)).toBe(hex);
    }
  });

  it("gives hue, saturation and value", () => {
    expect(rgbToHsv({ r: 255, g: 0, b: 0 })).toEqual({ h: 0, s: 1, v: 1 });
    const blue = rgbToHsv({ r: 0, g: 0, b: 128 });
    expect(blue.h).toBe(240);
    expect(blue.s).toBe(1);
    expect(blue.v).toBeCloseTo(128 / 255);
    expect(rgbToHsv({ r: 128, g: 128, b: 128 }).s).toBe(0);
    expect(rgbToHex({ r: 300, g: -4, b: 12.4 })).toBe("#ff000c");
  });
});
