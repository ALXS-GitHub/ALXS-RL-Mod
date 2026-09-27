import { Gamepad2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Page, PageHeader } from "@/components/ui/layout";
import { LaunchCards } from "./components/LaunchCards";

export function PlayPage() {
  const { t } = useTranslation("play");
  return (
    <Page>
      <PageHeader icon={Gamepad2} eyebrow={t("eyebrow")} title={t("title")} subtitle={t("subtitle")} />
      <LaunchCards />
    </Page>
  );
}
