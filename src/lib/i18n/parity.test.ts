import { describe, expect, it } from "vitest";
import commonEn from "./locales/en/common.json";
import commonFr from "./locales/fr/common.json";

/** Plural suffixes are language-specific; compare the base keys only. */
const PLURAL = /_(zero|one|two|few|many|other)$/;

function keys(obj: unknown, prefix = ""): string[] {
  if (typeof obj !== "object" || obj === null) return [prefix];
  return Object.entries(obj).flatMap(([k, v]) => keys(v, prefix ? `${prefix}.${k}` : k));
}

function baseKeys(obj: unknown): string[] {
  return [...new Set(keys(obj).map((k) => k.replace(PLURAL, "")))].sort();
}

const features = import.meta.glob<{ default: unknown }>("/src/features/*/locales/*.json", { eager: true });

function namespaces(): Map<string, { en?: unknown; fr?: unknown }> {
  const map = new Map<string, { en?: unknown; fr?: unknown }>([["common", { en: commonEn, fr: commonFr }]]);
  for (const [path, mod] of Object.entries(features)) {
    const m = /\/features\/([^/]+)\/locales\/(en|fr)\.json$/.exec(path);
    if (!m?.[1] || !m[2]) continue;
    const entry = map.get(m[1]) ?? {};
    entry[m[2] as "en" | "fr"] = mod.default;
    map.set(m[1], entry);
  }
  return map;
}

describe("i18n", () => {
  for (const [ns, { en, fr }] of namespaces()) {
    it(`${ns}: EN and FR define the same keys`, () => {
      expect(en, `${ns} has no en.json`).toBeDefined();
      expect(fr, `${ns} has no fr.json`).toBeDefined();
      expect(baseKeys(fr)).toEqual(baseKeys(en));
    });
  }
});
