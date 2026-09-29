import { Activity, Radio, TrendingUp, Trophy } from "lucide-react";
import { lazy, Suspense, useState } from "react";
import { useTranslation } from "react-i18next";
import { Page, PageHeader } from "@/components/ui/layout";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { SessionView } from "./components/SessionView";
import { ConnectionBadge, TrackerSettings } from "./components/TrackerSettings";

type Tab = "session" | "mmr" | "settings";

// Personal edition only (docs/PERSONAL_EDITION.md): dropped from public builds.
const MmrView = __ALXS_PERSONAL__
  ? lazy(() => import("@/personal/tracker").then((m) => ({ default: m.MmrView })))
  : null;

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
          {MmrView ? (
            <TabsTrigger value="mmr">
              <TrendingUp />
              {t("personal:tabs.mmr")}
            </TabsTrigger>
          ) : null}
          <TabsTrigger value="settings">
            <Radio />
            {t("tabs.settings")}
          </TabsTrigger>
        </TabsList>
        <TabsContent value="session">
          <SessionView onOpenTracker={() => setTab("settings")} />
        </TabsContent>
        {MmrView ? (
          <TabsContent value="mmr">
            <Suspense fallback={null}>
              <MmrView />
            </Suspense>
          </TabsContent>
        ) : null}
        <TabsContent value="settings">
          <TrackerSettings />
        </TabsContent>
      </Tabs>
    </Page>
  );
}
