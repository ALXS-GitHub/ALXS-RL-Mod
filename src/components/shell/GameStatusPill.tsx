import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Tooltip } from "@/components/ui/tooltip";
import { useGameStatus } from "@/lib/game";

/** Live game state in the title bar. */
export function GameStatusPill() {
  const { t } = useTranslation();
  const { data } = useGameStatus();

  if (!data) return null;
  if (!data.install) {
    return (
      <Badge tone="warning" dot>
        {t("game.notFound")}
      </Badge>
    );
  }
  const { running } = data;
  const source = t(`game.${data.install.source}`);
  if (!running.running) {
    return (
      <Tooltip content={data.install.root}>
        <Badge tone="neutral" dot>
          {t("game.idle")} · {source}
        </Badge>
      </Tooltip>
    );
  }
  return (
    <Badge tone={running.withEac ? "orange" : "success"} dot pulse>
      {running.foreground ? t("game.focused") : running.withEac ? t("game.runningEac") : t("game.running")}
    </Badge>
  );
}
