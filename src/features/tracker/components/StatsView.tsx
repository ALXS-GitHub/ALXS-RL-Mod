import { BarChart3 } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { EmptyState, ErrorState, Skeleton } from "@/components/ui/feedback";
import { Stat } from "@/components/ui/layout";
import { Segmented } from "@/components/ui/segmented";
import { Select } from "@/components/ui/select";
import { cn } from "@/lib/cn";
import { formatDateTime } from "@/lib/format";
import i18n from "@/lib/i18n";
import { arenaLabel } from "../api";
import {
  bestSessions,
  byArena,
  byDayPart,
  type Filters,
  filterMatches,
  type Group,
  KNOWN_PLAYLISTS,
  type Period,
  playlistsOf,
  resultsOverTime,
  rollingScore,
  summarize,
} from "../history";
import { useMatchHistory } from "../queries";
import { Key, LOSS_COLOR, ResultColumns, TrendLine, WIN_COLOR } from "./charts";

const ROLLING = 10;

const pct = (v: number | null) => (v === null ? "—" : `${Math.round(v * 100)} %`);
const one = (v: number) => (Math.round(v * 10) / 10).toLocaleString(i18n.language);

function useShortDate() {
  const { i18n: i } = useTranslation();
  const locale = i.language === "fr" ? "fr-FR" : "en-US";
  return (d: Date | string) =>
    new Intl.DateTimeFormat(locale, { day: "numeric", month: "short" }).format(
      typeof d === "string" ? new Date(d) : d,
    );
}

function usePlaylistName() {
  const { t } = useTranslation("tracker");
  return (id: number) =>
    (KNOWN_PLAYLISTS as readonly number[]).includes(id)
      ? t(`history.playlists.${id}`)
      : t("history.playlistId", { id });
}

function GroupTable({
  title,
  rows,
  label,
}: {
  title: string;
  rows: Group[];
  label: (key: string) => string;
}) {
  const { t } = useTranslation("tracker");
  if (!rows.length) return null;
  return (
    <div className="min-w-0 flex-1">
      <p className="mb-1.5 text-xs text-fg-subtle">{title}</p>
      <table className="w-full text-[13px]">
        <thead className="sr-only">
          <tr>
            <th>{title}</th>
            <th>{t("history.matches")}</th>
            <th>{t("history.record")}</th>
            <th>{t("history.winRate")}</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-[var(--color-line)]">
          {rows.map((g) => (
            <tr key={g.key} className="h-8">
              <td className="truncate pr-2">{label(g.key)}</td>
              <td className="w-16 text-right text-fg-muted tabular-nums">{g.matches}</td>
              <td className="w-20 text-right tabular-nums">
                {g.wins}–{g.losses}
              </td>
              <td className="w-16 text-right tabular-nums">{pct(g.winRate)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** The "Stats" tab: results and form over time, from the recorded matches. */
export function StatsView() {
  const { t } = useTranslation("tracker");
  const history = useMatchHistory();
  const shortDate = useShortDate();
  const playlistName = usePlaylistName();
  const [filters, setFilters] = useState<Filters>({ period: "30d", playlist: null });

  const all = history.data ?? [];
  const playlists = useMemo(() => playlistsOf(all), [all]);
  const matches = useMemo(() => filterMatches(all, filters), [all, filters]);
  const summary = useMemo(() => summarize(matches), [matches]);
  const overTime = useMemo(() => resultsOverTime(matches), [matches]);
  const form = useMemo(() => rollingScore(matches, ROLLING), [matches]);
  const arenas = useMemo(() => byArena(matches).slice(0, 6), [matches]);
  const dayParts = useMemo(() => byDayPart(matches), [matches]);
  const sessions = useMemo(() => bestSessions(matches), [matches]);

  if (history.isPending) return <Skeleton className="h-96" />;
  if (history.error) return <ErrorState error={history.error} onRetry={() => void history.refetch()} />;
  if (all.length === 0) {
    return <EmptyState icon={BarChart3} title={t("history.empty")} description={t("history.emptyHint")} />;
  }

  const periods: { value: Period; label: string }[] = (["7d", "30d", "90d", "all"] as const).map((p) => ({
    value: p,
    label: t(`history.periods.${p}`),
  }));
  const playlistOptions = [
    { value: "all", label: t("history.allPlaylists") },
    ...playlists.map((id) => ({ value: String(id), label: playlistName(id) })),
  ];

  return (
    <section className={cn("flex flex-col gap-5", history.isFetching && "opacity-80")}>
      {/* Filters: one row, above everything they scope. */}
      <div className="flex flex-wrap items-center gap-3">
        <Segmented
          value={filters.period}
          onValueChange={(period) => setFilters((f) => ({ ...f, period }))}
          options={periods}
          size="sm"
          aria-label={t("history.period")}
        />
        {playlists.length ? (
          <div className="w-52">
            <Select
              value={filters.playlist === null ? "all" : String(filters.playlist)}
              onValueChange={(v) => setFilters((f) => ({ ...f, playlist: v === "all" ? null : Number(v) }))}
              options={playlistOptions}
            />
          </div>
        ) : null}
        <p className="ml-auto text-xs text-fg-subtle">{t("history.source")}</p>
      </div>

      {matches.length === 0 ? (
        <p className="text-sm text-fg-muted">{t("history.noneInFilter")}</p>
      ) : (
        <>
          <div className="grid grid-cols-2 gap-x-6 gap-y-4 @2xl:grid-cols-4 @5xl:grid-cols-7">
            <Stat label={t("history.matches")} value={summary.matches} />
            <Stat
              label={t("history.winRate")}
              value={pct(summary.winRate)}
              hint={t("history.recordHint", { wins: summary.wins, losses: summary.losses })}
            />
            <Stat label={t("history.bestStreak")} value={summary.bestStreak} />
            <Stat label={t("history.avgScore")} value={summary.avg ? Math.round(summary.avg.score) : "—"} />
            <Stat label={t("history.avgGoals")} value={summary.avg ? one(summary.avg.goals) : "—"} />
            <Stat label={t("history.avgSaves")} value={summary.avg ? one(summary.avg.saves) : "—"} />
            <Stat label={t("history.mvps")} value={summary.mvps} />
          </div>

          <div className="grid gap-5 @5xl:grid-cols-2">
            <div className="rounded-lg border border-line bg-white/[0.02] p-3">
              <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
                <p className="text-[13px] font-medium">
                  {overTime.unit === "day" ? t("history.perDay") : t("history.perWeek")}
                </p>
                <div className="flex items-center gap-3 text-xs text-fg-muted">
                  <span className="flex items-center gap-1.5">
                    <Key color={WIN_COLOR} />
                    {t("history.wins")}
                  </span>
                  <span className="flex items-center gap-1.5">
                    <Key color={LOSS_COLOR} />
                    {t("history.losses")}
                  </span>
                </div>
              </div>
              <ResultColumns
                data={overTime.buckets.map((b) => ({
                  key: b.key,
                  label: shortDate(b.start),
                  wins: b.wins,
                  losses: b.losses,
                }))}
                winsLabel={t("history.wins")}
                lossesLabel={t("history.losses")}
                ariaLabel={overTime.unit === "day" ? t("history.perDay") : t("history.perWeek")}
              />
            </div>
            <div className="rounded-lg border border-line bg-white/[0.02] p-3">
              <p className="mb-2 text-[13px] font-medium">{t("history.form", { count: ROLLING })}</p>
              <TrendLine
                ariaLabel={t("history.form", { count: ROLLING })}
                data={form.map((p) => ({
                  key: String(p.index),
                  label: formatDateTime(p.endedAt),
                  value: Math.round(p.average),
                  details: [
                    [t("history.rollingAverage", { count: ROLLING }), String(Math.round(p.average))],
                    [t("history.matchScore"), String(p.value)],
                  ],
                }))}
              />
            </div>
          </div>

          <div className="flex flex-wrap gap-6">
            <GroupTable title={t("history.byArena")} rows={arenas} label={(k) => arenaLabel(k || null)} />
            <GroupTable
              title={t("history.byDayPart")}
              rows={dayParts}
              label={(k) => t(`history.dayParts.${k}`)}
            />
            {sessions.length ? (
              <div className="min-w-0 flex-1">
                <p className="mb-1.5 text-xs text-fg-subtle">{t("history.bestSessions")}</p>
                <ul className="divide-y divide-[var(--color-line)] text-[13px]">
                  {sessions.map((s) => (
                    <li key={s.start} className="flex h-8 items-center justify-between gap-2">
                      <span className="truncate">{formatDateTime(s.start)}</span>
                      <span className="tabular-nums">
                        {s.wins}–{s.losses}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            ) : null}
          </div>
        </>
      )}
    </section>
  );
}
