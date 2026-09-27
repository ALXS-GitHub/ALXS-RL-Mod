import { Link } from "@tanstack/react-router";
import {
  ArrowUpRight,
  Brush,
  FolderSearch,
  Gamepad2,
  KeyRound,
  Map as MapIcon,
  Palette,
  Rocket,
  ShieldCheck,
  Sparkles,
  Trophy,
} from "lucide-react";
import { motion } from "motion/react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { GlassCard } from "@/components/ui/glass";
import { Page, Stat } from "@/components/ui/layout";
import { useDecalStatus } from "@/features/decals/queries";
import { useSwaps } from "@/features/items/queries";
import { useMapSession } from "@/features/maps/queries";
import { usePaletteStatus } from "@/features/palette/queries";
import { useLaunch } from "@/features/play/queries";
import { useKeysStatus } from "@/features/settings/api";
import { useTrackerSession } from "@/features/tracker/queries";
import { useGameStatus } from "@/lib/game";

const stagger = {
  hidden: {},
  show: { transition: { staggerChildren: 0.04 } },
};
const rise = {
  hidden: { opacity: 0, y: 12 },
  show: { opacity: 1, y: 0, transition: { type: "spring" as const, stiffness: 260, damping: 26 } },
};

interface TileProps {
  to: string;
  icon: typeof Sparkles;
  title: string;
  value: ReactNode;
  detail: ReactNode;
  active: boolean;
}

function Tile({ to, icon: Icon, title, value, detail, active }: TileProps) {
  return (
    <motion.div variants={rise}>
      <Link to={to} className="block h-full">
        <GlassCard className="group flex h-full flex-col gap-3 p-4">
          <div className="flex items-center justify-between text-xs text-fg-subtle">
            <span className="flex items-center gap-1.5">
              <Icon className="size-3.5" />
              {title}
            </span>
            {active ? (
              <span className="size-1.5 rounded-full bg-[var(--color-accent)]" aria-hidden />
            ) : (
              <ArrowUpRight className="size-3.5 opacity-0 transition-opacity group-hover:opacity-100" />
            )}
          </div>
          <div className="min-w-0">
            <p className="truncate text-[15px] font-semibold">{value}</p>
            <p className="mt-0.5 truncate text-xs text-fg-muted">{detail}</p>
          </div>
        </GlassCard>
      </Link>
    </motion.div>
  );
}

export function HomePage() {
  const { t } = useTranslation("home");
  const game = useGameStatus();
  const keys = useKeysStatus();
  const swaps = useSwaps();
  const palette = usePaletteStatus();
  const decal = useDecalStatus();
  const mapSession = useMapSession();
  const session = useTrackerSession();
  const launch = useLaunch();

  const install = game.data?.install;
  const running = game.data?.running.running ?? false;
  const swapCount = swaps.data?.length ?? 0;
  const activePalette = palette.data?.active;
  const activeDecal = decal.data?.active;
  const map = mapSession.data;
  const s = session.data;
  const needsSetup = game.isSuccess && (!install || keys.data?.present === false);

  return (
    <Page className="gap-7">
      {/* Hero */}
      <section className="flex flex-wrap items-end justify-between gap-6 pt-6 pb-2">
        <div className="max-w-2xl">
          <p className="mb-3 flex items-center gap-1.5 text-xs text-fg-subtle">
            <ShieldCheck className="size-3.5" />
            {t("hero.safe")}
          </p>
          <h1 className="text-[34px] leading-[1.1] font-semibold tracking-[-0.03em]">{t("hero.title")}</h1>
          <p className="mt-2.5 max-w-xl text-sm text-fg-muted">{t("hero.subtitle")}</p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            variant="primary"
            size="lg"
            disabled={!install || running}
            loading={launch.isPending && launch.variables === true}
            onClick={() => launch.mutate(true)}
          >
            <Rocket />
            {t("hero.playOnline")}
          </Button>
          <Button
            size="lg"
            disabled={!install || running}
            loading={launch.isPending && launch.variables === false}
            onClick={() => launch.mutate(false)}
          >
            <Gamepad2 />
            {t("hero.playOffline")}
          </Button>
        </div>
      </section>

      {needsSetup ? (
        <GlassCard
          interactive={false}
          className="flex flex-wrap items-center justify-between gap-4 border-warning/25 p-4"
        >
          <div className="flex items-center gap-3">
            {!install ? (
              <FolderSearch className="size-4 text-warning" />
            ) : (
              <KeyRound className="size-4 text-warning" />
            )}
            <div>
              <p className="text-[13px] font-medium">
                {!install ? t("setup.gameTitle") : t("setup.keysTitle")}
              </p>
              <p className="text-[13px] text-fg-muted">
                {!install ? t("setup.gameBody") : t("setup.keysBody")}
              </p>
            </div>
          </div>
          <Button asChild variant="outline" size="sm">
            <Link to="/settings">{t("setup.cta")}</Link>
          </Button>
        </GlassCard>
      ) : null}

      {/* Current loadout */}
      <section className="flex flex-col gap-3">
        <h2 className="text-sm font-semibold">{t("loadout.title")}</h2>
        <motion.div
          variants={stagger}
          initial="hidden"
          animate="show"
          className="grid gap-4 @2xl:grid-cols-2 @5xl:grid-cols-4"
        >
          <Tile
            to="/items"
            icon={Sparkles}
            title={t("loadout.items")}
            active={swapCount > 0}
            value={t("loadout.swaps", { count: swapCount })}
            detail={
              swapCount > 0
                ? swaps.data
                    ?.map((w) => w.wantedLabel)
                    .slice(0, 3)
                    .join(" · ")
                : t("loadout.stock")
            }
          />
          <Tile
            to="/palette"
            icon={Palette}
            title={t("loadout.palette")}
            active={Boolean(activePalette)}
            value={activePalette?.paletteName ?? t("loadout.stockPalette")}
            detail={
              activePalette?.stale
                ? t("loadout.stale")
                : activePalette
                  ? t("loadout.applied")
                  : t("loadout.stock")
            }
          />
          <Tile
            to="/decals"
            icon={Brush}
            title={t("loadout.decal")}
            active={Boolean(activeDecal)}
            value={activeDecal?.displayName ?? t("loadout.none")}
            detail={activeDecal ? activeDecal.bodyName : t("loadout.stock")}
          />
          <Tile
            to="/maps"
            icon={MapIcon}
            title={t("loadout.map")}
            active={Boolean(map)}
            value={map?.mapName ?? t("loadout.none")}
            detail={map ? t("loadout.mapTarget", { target: map.target }) : t("loadout.stockMaps")}
          />
        </motion.div>
      </section>

      {/* Session */}
      <section className="flex flex-col gap-3">
        <div className="flex items-end justify-between">
          <h2 className="text-sm font-semibold">{t("session.title")}</h2>
          <Button asChild variant="ghost" size="sm">
            <Link to="/tracker">
              {t("session.open")}
              <ArrowUpRight />
            </Link>
          </Button>
        </div>
        <GlassCard interactive={false} className="grid gap-5 p-5 @2xl:grid-cols-2 @4xl:grid-cols-5">
          <Stat label={t("session.wins")} value={s?.wins ?? 0} tone="success" />
          <Stat label={t("session.losses")} value={s?.losses ?? 0} tone="danger" />
          <Stat
            label={t("session.streak")}
            value={s ? (s.streak > 0 ? `+${s.streak}` : s.streak) : 0}
            tone={s && s.streak > 0 ? "success" : s && s.streak < 0 ? "danger" : "default"}
          />
          <Stat label={t("session.goals")} value={s?.totals.goals ?? 0} />
          <Stat
            label={t("session.mvps")}
            value={
              <span className="inline-flex items-center gap-2">
                <Trophy className="size-5 text-warning" />
                {s?.totals.mvps ?? 0}
              </span>
            }
          />
        </GlassCard>
      </section>
    </Page>
  );
}
