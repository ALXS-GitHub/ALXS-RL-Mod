/**
 * Entry point of the personal edition for the tracker (one lazy chunk).
 * Public code imports it only behind `__ALXS_PERSONAL__`, e.g.
 * `__ALXS_PERSONAL__ ? lazy(() => import("@/personal/tracker").then((m) => ({ default: m.MmrView }))) : null`,
 * so the public build drops it entirely.
 */
import "./i18n";

export { MmrView } from "./components/MmrView";
export { OverlayRatings } from "./components/OverlayRatings";
export { SessionRatings } from "./components/SessionRatings";
