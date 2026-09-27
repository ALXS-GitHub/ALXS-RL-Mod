import { Activity, AppWindow } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/feedback";
import { Field } from "@/components/ui/layout";
import { Switch } from "@/components/ui/switch";
import type { Connection, StatsStatus } from "../api";
import { useStatsStatus, useToggleOverlay, useToggleStats } from "../queries";

/** What the tracker is doing, as the user sees it. */
export type TrackerState = Connection | "off" | "waiting";

export function trackerState(s: StatsStatus): TrackerState {
  if (!s.enabled) return "off";
  if (!s.gameRunning) return "waiting";
  return s.connection;
}

const stateTone: Record<TrackerState, "neutral" | "info" | "success" | "warning"> = {
  off: "neutral",
  waiting: "neutral",
  idle: "neutral",
  connecting: "info",
  connected: "success",
  unreachable: "warning",
};

/** Tracker state badge (Stats API off / waiting for the game / live…). */
export function ConnectionBadge() {
  const { t } = useTranslation("tracker");
  const { data: s } = useStatsStatus();
  if (!s) return null;
  const state = trackerState(s);
  return (
    <Badge tone={stateTone[state]} dot pulse={state === "connected" || state === "connecting"}>
      {t(`connection.${state}`)}
    </Badge>
  );
}

/** Stats API + overlay switches (the "Tracker" tab). */
export function TrackerSettings() {
  const { t } = useTranslation("tracker");
  const status = useStatsStatus();
  const toggleStats = useToggleStats();
  const toggleOverlay = useToggleOverlay();
  const s = status.data;

  return (
    <section className="flex max-w-2xl flex-col">
      {!s ? (
        <Skeleton className="h-28" />
      ) : (
        <div className="divide-y divide-[var(--color-line)]">
          <Field
            label={
              <span className="flex items-center gap-2">
                <Activity className="size-3.5 text-fg-subtle" />
                {t("setup.statsApi")}
              </span>
            }
            description={
              s.problem ? t("setup.noConfig") : s.enabled ? t("setup.statsOn") : t("setup.statsOff")
            }
          >
            <Switch
              checked={s.enabled}
              disabled={Boolean(s.problem) || toggleStats.isPending}
              onCheckedChange={(v) => toggleStats.mutate(v)}
              aria-label={t("setup.statsApi")}
            />
          </Field>
          <Field
            label={
              <span className="flex items-center gap-2">
                <AppWindow className="size-3.5 text-fg-subtle" />
                {t("setup.overlay")}
              </span>
            }
            description={t("setup.overlayHint")}
          >
            <Switch
              checked={s.overlayOpen}
              disabled={toggleOverlay.isPending}
              onCheckedChange={(v) => toggleOverlay.mutate(v)}
              aria-label={t("setup.overlay")}
            />
          </Field>
          {s.enabled && s.gameRunning && s.connection === "unreachable" ? (
            <p className="pt-3 text-xs text-warning">{t("setup.restartGame")}</p>
          ) : null}
          {s.iniPath ? (
            <p className="truncate pt-3 text-[11px] text-fg-subtle" title={s.iniPath} data-selectable>
              {t("setup.iniPath")} {s.iniPath}
            </p>
          ) : null}
        </div>
      )}
    </section>
  );
}
