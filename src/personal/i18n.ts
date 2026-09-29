import i18n from "@/lib/i18n";
import en from "./locales/en.json";
import fr from "./locales/fr.json";

/**
 * Registers the `personal` namespace. Its strings live outside
 * `src/features/*` on purpose: the public bundle must not contain them, so
 * they are loaded only when a personal module is (lazily) imported.
 */
if (!i18n.hasResourceBundle("en", "personal")) {
  for (const [lng, bundle] of [
    ["en", en],
    ["fr", fr],
  ] as const) {
    i18n.addResourceBundle(lng, "personal", bundle);
    // Backend error codes are translated from `common:errors.<code>`.
    i18n.addResourceBundle(lng, "common", { errors: bundle.errors }, true, false);
  }
}
