import { Puzzle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Page, PageHeader } from "@/components/ui/layout";
import { BakkesCard } from "./components/BakkesCard";
import { ReplaysPanel } from "./components/ReplaysPanel";

export function ExtrasPage() {
  const { t } = useTranslation("extras");
  return (
    <Page>
      <PageHeader icon={Puzzle} eyebrow={t("eyebrow")} title={t("title")} subtitle={t("subtitle")} />
      <div className="grid gap-4 @5xl:grid-cols-[minmax(0,1fr)_360px]">
        <ReplaysPanel />
        <BakkesCard />
      </div>
    </Page>
  );
}
