import { lazy, StrictMode, Suspense } from "react";
import { createRoot } from "react-dom/client";
import "./styles/globals.css";
import "./lib/i18n";
import { App } from "./app/App";

/**
 * Two windows share this bundle:
 * - the main app (default),
 * - the match tracker overlay, opened by the backend at `index.html?window=overlay`.
 * The overlay skips the router, the shell and the WebGL layer entirely.
 */
const OverlayRoot = lazy(() =>
  import("./features/tracker/OverlayRoot").then((m) => ({ default: m.OverlayRoot })),
);

const isOverlay = new URLSearchParams(window.location.search).get("window") === "overlay";
const root = document.getElementById("root");
if (!root) throw new Error("#root missing");

createRoot(root).render(
  <StrictMode>
    {isOverlay ? (
      <Suspense fallback={null}>
        <OverlayRoot />
      </Suspense>
    ) : (
      <App />
    )}
  </StrictMode>,
);
