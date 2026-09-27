import { Crown, Flame, Info, RotateCcw, Snowflake, Trophy } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { EmptyState, Skeleton } from "@/components/ui/feedback";
import { Stat } from "@/components/ui/layout";
import { Tooltip } from "@/components/ui/tooltip";
import { cn } from "@/lib/cn";
import { formatDateTime, formatRelative } from "@/lib/format";
import {
  arenaLabel,
  formatClock,
  type LiveMatch,
  type MatchRecord,
  type SessionState,
  topMmrDelta,
} from "../api";
import { useResetSession, useStatsStatus, useTrackerSession } from "../queries";
import { LobbyPanel } from "./LobbyPanel";

function WinRing({ wins, losses }: { wins: number; losses: number }) {
  const { t } = useTranslation("tracker");
  const total = wins + losses;
  const rate = total ? wins / total : 0;
  const r = 52;
  const c = 2 * Math.PI * r;
  return (
    <div className="relative grid size-28 place-items-center">
      <svg viewBox="0 0 128 128" className="absolute inset-0 -rotate-90" aria-hidden>
        <circle cx="64" cy="64" r={r} fill="none" stroke="rgb(255 255 255 / 0.07)" strokeWidth="6" />
        <motion.circle
          cx="64"
          cy="64"
          r={r}
          fill="none"
          stroke="var(--color-success)"
          strokeWidth="6"
          strokeLinecap="round"
          strokeDasharray={c}
          initial={{ strokeDashoffset: c }}
          animate={{ strokeDashoffset: c * (1 - rate) }}
          transition={{ type: "spring", stiffness: 60, damping: 18 }}
        />
      </svg>
      <div className="text-center">
        <p className="text-[22px] font-semibold tabular-nums">
          <span className="text-success">{wins}</span>
          <span className="mx-1 text-fg-subtle">–</span>
          <span className="text-danger">{losses}</span>
        </p>
        <p className="text-xs text-fg-subtle">
          {total ? t("session.winRate", { rate: Math.round(rate * 100) }) : t("session.noMatches")}
        </p>
      </div>
    </div>
  );
}

function StreakBadge({ streak }: { streak: number }) {
  const { t } = useTranslation("tracker");
  if (streak === 0) return <Badge>{t("session.noStreak")}</Badge>;
  const hot = streak > 0;
  return (
    <Badge tone={hot ? "orange" : "info"} className="text-xs">
      {hot ? <Flame /> : <Snowflake />}
      {hot ? t("session.winStreak", { count: streak }) : t("session.lossStreak", { count: -streak })}
    </Badge>
  );
}

function LiveScoreboard({ live }: { live: LiveMatch }) {
  const { t } = useTranslation("tracker");
  const blueWin = live.blueScore > live.orangeScore;
  const orangeWin = live.orangeScore > live.blueScore;
  return (
    <div className="glass-inset relative overflow-hidden rounded-lg p-4">
      <div className="absolute inset-y-0 left-0 w-0.5 bg-team-blue" />
      <div className="absolute inset-y-0 right-0 w-0.5 bg-team-orange" />
      <div className="relative flex items-center justify-between gap-4">
        <div className="flex flex-col">
          <span className="text-xs text-[#9cc1ff]">
            {t("live.blue")}
            {live.yourTeam === 0 ? ` · ${t("live.you")}` : ""}
          </span>
          <motion.span
            key={live.blueScore}
            initial={{ opacity: 0.4 }}
            animate={{ opacity: 1 }}
            className={cn("text-[28px] font-semibold tabular-nums", blueWin && "text-[#9cc1ff]")}
          >
            {live.blueScore}
          </motion.span>
        </div>
        <div className="flex flex-col items-center gap-1">
          <Badge tone="danger" dot pulse>
            {live.replay ? t("live.replay") : t("live.live")}
          </Badge>
          <span className="text-lg font-semibold tabular-nums">
            {live.overtime ? "+" : ""}
            {formatClock(live.timeSeconds)}
          </span>
          <span className="text-xs text-fg-subtle">{arenaLabel(live.arena)}</span>
        </div>
        <div className="flex flex-col items-end">
          <span className="text-xs text-[#ffbe8f]">
            {live.yourTeam === 1 ? `${t("live.you")} · ` : ""}
            {t("live.orange")}
          </span>
          <motion.span
            key={live.orangeScore}
            initial={{ opacity: 0.4 }}
            animate={{ opacity: 1 }}
            className={cn("text-[28px] font-semibold tabular-nums", orangeWin && "text-[#ffbe8f]")}
          >
            {live.orangeScore}
          </motion.span>
        </div>
      </div>
      {live.you ? (
        <div className="relative mt-3 flex flex-wrap gap-x-5 gap-y-1 border-line border-t pt-3 text-xs text-fg-muted tabular-nums">
          <span className="font-medium text-fg">{live.you.name}</span>
          <span>
            {t("stats.score")} {live.you.score}
          </span>
          <span>
            {t("stats.goals")} {live.you.goals}
          </span>
          <span>
            {t("stats.assists")} {live.you.assists}
          </span>
          <span>
            {t("stats.saves")} {live.you.saves}
          </span>
          <span>
            {t("stats.shots")} {live.you.shots}
          </span>
        </div>
      ) : null}
    </div>
  );
}

function MatchPill({ m }: { m: MatchRecord }) {
  const { t } = useTranslation("tracker");
  const tone =
    m.result === "win"
      ? "border-success/40 bg-success/15 text-success"
      : m.result === "loss"
        ? "border-danger/40 bg-danger/15 text-danger"
        : "border-line bg-white/5 text-fg-subtle";
  const mine = m.yourTeam === 1 ? m.orangeScore : m.blueScore;
  const theirs = m.yourTeam === 1 ? m.blueScore : m.orangeScore;
  return (
    <Tooltip
      content={
        <div className="space-y-0.5">
          <p className="font-medium">{arenaLabel(m.arena)}</p>
          <p className="text-fg-muted">{formatDateTime(m.endedAt)}</p>
          {m.you ? (
            <p className="text-fg-muted">
              {t("stats.score")} {m.you.score} · {m.you.goals}G {m.you.assists}A {m.you.saves}S
            </p>
          ) : null}
        </div>
      }
    >
      <motion.div
        layout
        initial={{ opacity: 0, scale: 0.6 }}
        animate={{ opacity: 1, scale: 1 }}
        className={cn(
          "relative flex h-12 w-11 shrink-0 flex-col items-center justify-center rounded-[7px] border text-[13px] font-semibold tabular-nums",
          tone,
        )}
      >
        {m.mvp ? <Crown className="absolute -top-2 size-3 text-warning" /> : null}
        <span className="text-[10px] font-medium">{t(`result.${m.result}`)}</span>
        <span>
          {mine}-{theirs}
          {m.overtime ? "*" : ""}
        </span>
      </motion.div>
    </Tooltip>
  );
}

function SessionBody({ s }: { s: SessionState }) {
  const { t } = useTranslation("tracker");
  const mmr = topMmrDelta(s.mmr);
  const played = s.matches.length;
  return (
    <div className="flex flex-col gap-5">
      <div className="flex flex-wrap items-center gap-6">
        <WinRing wins={s.wins} losses={s.losses} />
        <div className="grid flex-1 grid-cols-2 gap-x-6 gap-y-4 @2xl:grid-cols-4">
          <Stat label={t("stats.goals")} value={s.totals.goals} />
          <Stat label={t("stats.assists")} value={s.totals.assists} />
          <Stat label={t("stats.saves")} value={s.totals.saves} />
          <Stat label={t("stats.mvps")} value={s.totals.mvps} tone={s.totals.mvps ? "accent" : "default"} />
          <Stat label={t("stats.bestStreak")} value={s.bestStreak} hint={<StreakBadge streak={s.streak} />} />
          <Stat label={t("stats.avgScore")} value={played ? Math.round(s.totals.score / played) : "—"} />
          <Stat label={t("stats.shots")} value={s.totals.shots} />
          <Stat
            label={t("stats.mmr")}
            value={mmr ? `${mmr.delta >= 0 ? "+" : ""}${mmr.delta}` : "—"}
            tone={mmr ? (mmr.delta >= 0 ? "success" : "danger") : "default"}
            hint={mmr ? `${mmr.playlist} · ${mmr.latest}` : t("stats.mmrHint")}
          />
        </div>
      </div>

      <AnimatePresence>
        {s.live && !s.live.ended ? (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
          >
            <LiveScoreboard live={s.live} />
          </motion.div>
        ) : null}
      </AnimatePresence>

      <LobbyPanel lobby={s.lobby} />

      <div>
        <div className="mb-2 flex items-center justify-between">
          <p className="text-xs text-fg-subtle">{t("session.timeline")}</p>
          <p className="text-xs text-fg-subtle">
            {t("session.since", { when: formatRelative(s.startedAt) })}
          </p>
        </div>
        {played ? (
          <div className="flex gap-2 overflow-x-auto pt-2 pb-1">
            {s.matches.slice(0, 30).map((m, i) => (
              <MatchPill key={m.guid ?? `${m.endedAt}-${i}`} m={m} />
            ))}
          </div>
        ) : (
          <p className="text-sm text-fg-muted">{t("session.timelineEmpty")}</p>
        )}
      </div>
    </div>
  );
}

/** The "Session" tab: this session's record, live match, lobby and matches. */
export function SessionView({ onOpenTracker }: { onOpenTracker: () => void }) {
  const { t } = useTranslation("tracker");
  const session = useTrackerSession();
  const reset = useResetSession();
  const { data: stats } = useStatsStatus();

  return (
    <section className="flex flex-col gap-5">
      <div className="flex flex-wrap items-center gap-3">
        {session.data?.youName ? <Badge>{session.data.youName}</Badge> : null}
        {stats && !stats.enabled ? (
          <p className="flex items-center gap-2 text-xs text-fg-muted">
            <Info className="size-3.5 shrink-0" />
            {t("session.trackerOff")}
            <button
              type="button"
              onClick={onOpenTracker}
              className="text-fg underline-offset-2 hover:underline"
            >
              {t("session.trackerOffAction")}
            </button>
          </p>
        ) : null}
        <Button
          size="sm"
          variant="ghost"
          className="ml-auto"
          onClick={() => reset.mutate()}
          loading={reset.isPending}
        >
          <RotateCcw />
          {t("session.reset")}
        </Button>
      </div>
      {session.isPending ? (
        <Skeleton className="h-64" />
      ) : session.data ? (
        <SessionBody s={session.data} />
      ) : (
        <EmptyState icon={Trophy} title={t("session.unavailable")} />
      )}
    </section>
  );
}
