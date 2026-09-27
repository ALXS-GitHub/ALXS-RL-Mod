import i18n, { type Resource } from "i18next";
import { initReactI18next } from "react-i18next";
import commonEn from "./locales/en/common.json";
import commonFr from "./locales/fr/common.json";

export const SUPPORTED_LOCALES = ["en", "fr"] as const;
export type Locale = (typeof SUPPORTED_LOCALES)[number];

/**
 * Feature namespaces are discovered at build time: every
 * `src/features/<name>/locales/<lng>.json` becomes namespace `<name>`.
 * Features never touch this file to add strings.
 */
const featureFiles = import.meta.glob<{ default: Record<string, unknown> }>(
  "/src/features/*/locales/*.json",
  {
    eager: true,
  },
);

function buildResources(): Resource {
  const resources: Resource = { en: { common: commonEn }, fr: { common: commonFr } };
  for (const [path, mod] of Object.entries(featureFiles)) {
    const match = /\/features\/([^/]+)\/locales\/([a-z]{2})\.json$/.exec(path);
    if (!match) continue;
    const [, ns, lng] = match;
    if (!ns || !lng) continue;
    resources[lng] ??= {};
    const bucket = resources[lng];
    if (bucket) bucket[ns] = mod.default;
  }
  return resources;
}

void i18n.use(initReactI18next).init({
  resources: buildResources(),
  lng: "en",
  fallbackLng: "en",
  defaultNS: "common",
  ns: ["common"],
  interpolation: { escapeValue: false },
  returnNull: false,
});

export function setLocale(lng: Locale) {
  void i18n.changeLanguage(lng);
  document.documentElement.lang = lng;
}

export default i18n;
