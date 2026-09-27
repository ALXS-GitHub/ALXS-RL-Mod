import { Outlet, useRouterState } from "@tanstack/react-router";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { modifiesGame } from "@/app/nav";
import { AuroraCanvas } from "@/components/fx/AuroraCanvas";
import { notify, Toaster } from "@/components/ui/toast";
import { EVENTS, useTauriEvent } from "@/lib/events";
import { integrityReportSchema } from "@/lib/integrity";
import { useUi } from "@/stores/ui";
import { ActiveModsPanel } from "./ActiveModsPanel";
import { CommandPalette } from "./CommandPalette";
import { FxBridge } from "./FxBridge";
import { Sidebar } from "./Sidebar";
import { TitleBar } from "./TitleBar";
import { UpdateDialog } from "./UpdateDialog";
import { WelcomeDialog } from "./WelcomeDialog";

export function AppShell() {
  const { t } = useTranslation();
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const setModsPanelOpen = useUi((s) => s.setModsPanelOpen);

  // The active mods panel unfolds on pages that change game files, folds elsewhere.
  useEffect(() => setModsPanelOpen(modifiesGame(pathname)), [pathname, setModsPanelOpen]);

  // Background integrity checks (startup / after a game update) surface here.
  useTauriEvent(EVENTS.integrityReport, integrityReportSchema, (report) => {
    if (report.reapplied.length > 0) {
      notify.success(t("integrity.reapplied", { count: report.reapplied.length }));
    } else if (report.pending.length > 0) {
      notify.info(t("integrity.pending"));
    }
    for (const f of report.failures)
      notify.error({ kind: "Internal", message: `${f.feature}: ${f.message}` });
  });

  return (
    <div className="flex h-full flex-col">
      <AuroraCanvas />
      <div className="grain" aria-hidden />
      <FxBridge />
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        <Sidebar />
        <main key={pathname} className="@container relative min-w-0 flex-1 overflow-y-auto overflow-x-hidden">
          <Outlet />
        </main>
        <ActiveModsPanel />
        <WelcomeDialog />
        <UpdateDialog />
      </div>
      <CommandPalette />
      <Toaster />
    </div>
  );
}
