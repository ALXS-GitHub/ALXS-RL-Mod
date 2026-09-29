/**
 * `true` only in the personal edition (`ALXS_EDITION=personal`, see
 * docs/PERSONAL_EDITION.md). Replaced by a literal at build time (vite
 * `define`): use it directly in conditions so dead code is removed.
 */
declare const __ALXS_PERSONAL__: boolean;
