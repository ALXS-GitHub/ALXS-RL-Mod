import { MapPinned, RotateCcw } from "lucide-react";
import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { notify } from "@/components/ui/toast";
import { formatRelative } from "@/lib/format";
import { useGameStatus } from "@/lib/game";
import { targetLabel } from "../api";
import { useDeactivateMap, useMapSession } from "../queries";

/** Shown whenever a custom map is installed in a Labs slot. */
export function SessionBanner() {
  const { t } = useTranslation("maps");
  const { data: session } = useMapSession();
  const { data: game } = useGameStatus();
  const deactivate = useDeactivateMap();

  if (!session) return null;
  const running = game?.running.running ?? false;

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className="rounded-lg border border-[color-mix(in_oklab,var(--color-accent)_50%,transparent)] bg-[color-mix(in_oklab,var(--color-accent)_7%,var(--color-surface))] px-4 py-3"
    >
      <div className="flex flex-wrap items-center gap-3">
        <MapPinned className="size-4 shrink-0 text-[var(--color-accent)]" />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <p className="truncate text-sm font-semibold">{session.mapName}</p>
            <Badge tone="accent" dot pulse>
              {t("session.active")}
            </Badge>
            {session.offline ? <Badge tone="blue">{t("session.offline")}</Badge> : null}
          </div>
          <p className="mt-0.5 text-xs text-fg-muted">
            {t("session.details", {
              target: targetLabel(session.target),
              when: formatRelative(session.activatedAt),
            })}
            {session.offline ? ` · ${t("session.autoRestore")}` : ""}
          </p>
        </div>
        <Button
          variant="secondary"
          onClick={() =>
            deactivate.mutate(undefined, {
              onSuccess: () => {
                if (running) notify.success(t("toast.restoredLive"), t("toast.liveHint"));
              },
            })
          }
          loading={deactivate.isPending}
        >
          <RotateCcw />
          {t("session.restore")}
        </Button>
      </div>
    </motion.div>
  );
}
