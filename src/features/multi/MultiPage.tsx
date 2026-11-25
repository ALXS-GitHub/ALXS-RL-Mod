import { ShieldAlert, Users } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Page, PageHeader } from "@/components/ui/layout";
import { HostCard, JoinCard } from "./components/PlayCards";
import { SecurityPanel } from "./components/SecurityPanel";
import { SessionPanel } from "./components/SessionPanel";
import { SetupPanel } from "./components/SetupPanel";
import { useMultiSession, useMultiStatus } from "./queries";

export function MultiPage() {
  const { t } = useTranslation("multi");
  const status = useMultiStatus();
  const session = useMultiSession();
  const s = session.data;
  const busy = Boolean(s && s.phase !== "commandSent" && s.phase !== "manual" && s.phase !== "failed");

  return (
    <Page>
      <PageHeader icon={Users} eyebrow={t("eyebrow")} title={t("title")} subtitle={t("subtitle")} />
      <div className="flex items-start gap-2.5 rounded-[8px] border border-warning/25 bg-warning/[0.06] p-2.5 text-xs text-fg-muted">
        <ShieldAlert className="mt-0.5 size-3.5 shrink-0 text-warning" />
        <p>{t("offlineNote")}</p>
      </div>
      {s ? <SessionPanel session={s} /> : null}
      <div className="grid gap-4 @4xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,0.9fr)]">
        <HostCard status={status.data} busy={busy} />
        <JoinCard status={status.data} busy={busy} />
        <SetupPanel />
      </div>
      <SecurityPanel />
    </Page>
  );
}
