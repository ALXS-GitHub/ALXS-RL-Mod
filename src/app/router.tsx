import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  lazyRouteComponent,
  Navigate,
  Outlet,
} from "@tanstack/react-router";
import { AppShell } from "@/components/shell/AppShell";

/**
 * Code-based routes (no generator step). Each feature page is lazy-loaded
 * from `features/<name>/<Name>Page.tsx` with a named export.
 * Hash history: the app is served from a single `index.html` by Tauri.
 */

const rootRoute = createRootRoute({ component: Outlet });

const shellRoute = createRoute({ getParentRoute: () => rootRoute, id: "shell", component: AppShell });

const page = <P extends string>(
  path: P,
  loader: Parameters<typeof lazyRouteComponent>[0],
  exportName: string,
) =>
  createRoute({ getParentRoute: () => shellRoute, path, component: lazyRouteComponent(loader, exportName) });

const routes = [
  page("/", () => import("@/features/home/HomePage"), "HomePage"),
  page("/items", () => import("@/features/items/ItemsPage"), "ItemsPage"),
  page("/presets", () => import("@/features/presets/PresetsPage"), "PresetsPage"),
  page("/palette", () => import("@/features/palette/PalettePage"), "PalettePage"),
  page("/decals", () => import("@/features/decals/DecalsPage"), "DecalsPage"),
  page("/ball", () => import("@/features/ball/BallPage"), "BallPage"),
  page("/market", () => import("@/features/market/MarketPage"), "MarketPage"),
  page("/maps", () => import("@/features/maps/MapsPage"), "MapsPage"),
  page("/play", () => import("@/features/play/PlayPage"), "PlayPage"),
  page("/tracker", () => import("@/features/tracker/TrackerPage"), "TrackerPage"),
  page("/extras", () => import("@/features/extras/ExtrasPage"), "ExtrasPage"),
  page("/settings", () => import("@/features/settings/SettingsPage"), "SettingsPage"),
  page("/logs", () => import("@/features/logs/LogsPage"), "LogsPage"),
];

const routeTree = rootRoute.addChildren([shellRoute.addChildren(routes)]);

export const router = createRouter({
  routeTree,
  history: createHashHistory(),
  defaultPreload: "intent",
  defaultPendingMs: 150,
  // Unknown hash (stale link, typo): go home instead of a bare "Not Found".
  defaultNotFoundComponent: () => <Navigate to="/" />,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
