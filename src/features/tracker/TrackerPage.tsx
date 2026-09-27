import { Activity, Radio, TrendingUp, Trophy } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Page, PageHeader } from "@/components/ui/layout";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { MmrView } from "./components/MmrView";
import { SessionView } from "./components/SessionView";
import { ConnectionBadge, TrackerSettings } from "./components/TrackerSettings";

type Tab = "session" | "mmr" | "settings";

export function TrackerPage() {
  const { t } = useTranslation("tracker");
  const [tab, setTab] = useState<Tab>("session");
  return (
    <Page>
      <PageHeader
        icon={Activity}
        eyebrow={t("eyebrow")}
        title={t("title")}
        subtitle={t("subtitle")}
        actions={<ConnectionBadge />}
      />
      <Tabs value={tab} onValueChange={(v) => setTab(v as Tab)}>
        <TabsList>
          <TabsTrigger value="session">
            <Trophy />
            {t("tabs.session")}
          </TabsTrigger>
          <TabsTrigger value="mmr">
            <TrendingUp />
            {t("tabs.mmr")}
          </TabsTrigger>
          <TabsTrigger value="settings">
            <Radio />
            {t("tabs.settings")}
          </TabsTrigger>
        </TabsList>
        <TabsContent value="session">
          <SessionView onOpenTracker={() => setTab("settings")} />
        </TabsContent>
        <TabsContent value="mmr">
          <MmrView />
        </TabsContent>
        <TabsContent value="settings">
          <TrackerSettings />
        </TabsContent>
      </Tabs>
    </Page>
  );
}
