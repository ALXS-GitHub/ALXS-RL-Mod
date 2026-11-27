import { Info, LogIn, Radio, ShieldAlert, Users } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Page, PageHeader } from "@/components/ui/layout";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { GuidePanel } from "./components/GuidePanel";
import { HostTab } from "./components/host/HostTab";
import { JoinTab } from "./components/JoinTab";
import { SecurityPanel } from "./components/SecurityPanel";
import { SessionPanel } from "./components/SessionPanel";
import { isReady, SetupPanel } from "./components/SetupPanel";
import { useMultiSession, useMultiStatus } from "./queries";

type Tab = "host" | "join" | "infos";

export function MultiPage() {
  const { t } = useTranslation("multi");
  const status = useMultiStatus();
  const session = useMultiSession();
  const [tab, setTab] = useState<Tab>("host");
  const s = session.data;
  const busy = Boolean(s && !["commandSent", "hostInRocketPlugin", "manual", "failed"].includes(s.phase));
  const notReady = status.data ? !isReady(status.data) : false;
  const showInfos = () => setTab("infos");

  return (
    <Page>
      <PageHeader icon={Users} eyebrow={t("eyebrow")} title={t("title")} subtitle={t("subtitle")} />
      {s ? <SessionPanel session={s} /> : null}

      <Tabs value={tab} onValueChange={(v) => setTab(v as Tab)}>
        <TabsList>
          <TabsTrigger value="host">
            <Radio />
            {t("tabs.host")}
          </TabsTrigger>
          <TabsTrigger value="join">
            <LogIn />
            {t("tabs.join")}
          </TabsTrigger>
          <TabsTrigger value="infos">
            <Info />
            {t("tabs.infos")}
            {notReady ? <span className="size-1.5 rounded-full bg-warning" /> : null}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="host">
          <HostTab status={status.data} busy={busy} onShowInfos={showInfos} />
        </TabsContent>
        <TabsContent value="join">
          <JoinTab status={status.data} busy={busy} onShowInfos={showInfos} />
        </TabsContent>
        <TabsContent value="infos" className="flex flex-col gap-4">
          <div className="flex items-start gap-2.5 rounded-[8px] border border-warning/25 bg-warning/[0.06] p-2.5 text-xs text-fg-muted">
            <ShieldAlert className="mt-0.5 size-3.5 shrink-0 text-warning" />
            <p>{t("offlineNote")}</p>
          </div>
          <div className="grid items-start gap-4 @4xl:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
            <SetupPanel />
            <div className="flex flex-col gap-4">
              <GuidePanel defaultOpen />
              <SecurityPanel />
            </div>
          </div>
        </TabsContent>
      </Tabs>
    </Page>
  );
}
