import { Globe, type LucideIcon, Rocket, ShieldCheck, Wrench } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { GlassCard } from "@/components/ui/glass";
import { notify } from "@/components/ui/toast";
import { useMapSession } from "@/features/maps/queries";
import { useGameStatus } from "@/lib/game";
import { useLaunch } from "../queries";

interface LaunchCardProps {
  team: "orange" | "blue";
  icon: LucideIcon;
  title: string;
  description: string;
  points: string[];
  cta: string;
  disabledReason: string | null;
  /** Neutral information under the button (never blocks the launch). */
  note?: string | null;
  loading: boolean;
  onLaunch: () => void;
  badge: string;
}

function LaunchCard({
  team,
  icon: Icon,
  title,
  description,
  points,
  cta,
  disabledReason,
  note,
  loading,
  onLaunch,
  badge,
}: LaunchCardProps) {
  const color = team === "orange" ? "var(--color-team-orange)" : "var(--color-team-blue)";
  return (
    <GlassCard className="p-4" style={{ ["--color-accent" as string]: color }}>
      <div className="flex h-full flex-col gap-3">
        <div className="flex items-center justify-between gap-3">
          <h3 className="flex items-center gap-2 text-sm font-semibold">
            <Icon className="size-4" style={{ color }} />
            {title}
          </h3>
          <Badge tone={team}>{badge}</Badge>
        </div>
        <p className="text-[13px] text-fg-muted">{description}</p>
        <ul className="flex flex-col gap-1.5 text-xs text-fg-muted">
          {points.map((p) => (
            <li key={p} className="flex items-start gap-2">
              <span className="mt-1.5 size-1 shrink-0 rounded-full bg-fg-subtle" />
              {p}
            </li>
          ))}
        </ul>
        <div className="mt-auto flex flex-col gap-2">
          <Button
            variant="primary"
            className="w-full"
            onClick={onLaunch}
            loading={loading}
            disabled={Boolean(disabledReason)}
          >
            <Rocket />
            {cta}
          </Button>
          {disabledReason ? (
            <p className="text-center text-xs text-warning">{disabledReason}</p>
          ) : note ? (
            <p className="text-center text-xs text-fg-subtle">{note}</p>
          ) : null}
        </div>
      </div>
    </GlassCard>
  );
}

export function LaunchCards() {
  const { t } = useTranslation("play");
  const { data: game } = useGameStatus();
  const { data: session } = useMapSession();
  const launch = useLaunch();

  const running = game?.running.running ?? false;
  const missing = !game?.install;
  const common = missing ? t("launch.notInstalled") : running ? t("launch.alreadyRunning") : null;
  const store = game?.install?.source;

  const start = (withEac: boolean) =>
    launch.mutate(withEac, {
      onSuccess: (r) =>
        notify.success(
          r.method === "direct" ? t("launch.started") : t("launch.viaLauncher"),
          withEac ? t("launch.eacHint") : t("launch.offlineHint"),
        ),
    });

  return (
    <div className="grid gap-4 @3xl:grid-cols-2">
      <LaunchCard
        team="orange"
        icon={Globe}
        badge={t("launch.eac.badge")}
        title={t("launch.eac.title")}
        description={
          store === "steam"
            ? t("launch.eac.viaSteam")
            : store === "epic"
              ? t("launch.eac.viaEpic")
              : t("launch.eac.direct")
        }
        points={[t("launch.eac.p1"), t("launch.eac.p2"), t("launch.eac.p3")]}
        cta={t("launch.eac.cta")}
        disabledReason={common}
        note={session ? t("launch.eac.mapNote", { name: session.mapName }) : null}
        loading={launch.isPending && launch.variables === true}
        onLaunch={() => start(true)}
      />
      <LaunchCard
        team="blue"
        icon={Wrench}
        badge={t("launch.offline.badge")}
        title={t("launch.offline.title")}
        description={t("launch.offline.description")}
        points={[t("launch.offline.p1"), t("launch.offline.p2"), t("launch.offline.p3")]}
        cta={t("launch.offline.cta")}
        disabledReason={common}
        loading={launch.isPending && launch.variables === false}
        onLaunch={() => start(false)}
      />
      <p className="flex items-center gap-2 text-xs text-fg-subtle @3xl:col-span-2">
        <ShieldCheck className="size-3.5 text-success" />
        {t("launch.footnote")}
      </p>
    </div>
  );
}
