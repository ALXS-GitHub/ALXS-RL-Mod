/**
 * Match history maths for the "Stats" tab. Pure functions over the records
 * of `tracker_history` (oldest first); unit-tested in `history.test.ts`.
 */
import type { MatchRecord } from "./api";

export type Period = "7d" | "30d" | "90d" | "all";

export const PERIOD_DAYS: Record<Period, number | null> = { "7d": 7, "30d": 30, "90d": 90, all: null };

/** Playlists the Stats API reports (`PlaylistId`); others show their id. */
export const KNOWN_PLAYLISTS = [1, 2, 3, 4, 6, 10, 11, 13, 15, 17, 18, 22, 23, 27, 28, 29, 30, 34] as const;

export interface Filters {
  period: Period;
  /** `null` = every playlist. */
  playlist: number | null;
}

export function filterMatches(records: readonly MatchRecord[], f: Filters, now = new Date()): MatchRecord[] {
  const days = PERIOD_DAYS[f.period];
  const since = days === null ? null : now.getTime() - days * 86_400_000;
  return records.filter(
    (r) =>
      (since === null || Date.parse(r.endedAt) >= since) &&
      (f.playlist === null || r.playlistId === f.playlist),
  );
}

/** Playlists present in the records, most played first. */
export function playlistsOf(records: readonly MatchRecord[]): number[] {
  const counts = new Map<number, number>();
  for (const r of records) {
    if (r.playlistId != null) counts.set(r.playlistId, (counts.get(r.playlistId) ?? 0) + 1);
  }
  return [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([id]) => id);
}

export interface Summary {
  matches: number;
  wins: number;
  losses: number;
  /** Wins / decided matches, 0..1 (`null` without a decided match). */
  winRate: number | null;
  mvps: number;
  bestStreak: number;
  avg: { score: number; goals: number; assists: number; saves: number; shots: number } | null;
}

export function summarize(records: readonly MatchRecord[]): Summary {
  let wins = 0;
  let losses = 0;
  let mvps = 0;
  let streak = 0;
  let bestStreak = 0;
  const sum = { score: 0, goals: 0, assists: 0, saves: 0, shots: 0 };
  let withStats = 0;
  for (const r of records) {
    if (r.result === "win") {
      wins++;
      streak++;
      bestStreak = Math.max(bestStreak, streak);
    } else if (r.result === "loss") {
      losses++;
      streak = 0;
    }
    if (r.mvp) mvps++;
    if (r.you) {
      withStats++;
      sum.score += r.you.score;
      sum.goals += r.you.goals;
      sum.assists += r.you.assists;
      sum.saves += r.you.saves;
      sum.shots += r.you.shots;
    }
  }
  const decided = wins + losses;
  const per = (v: number) => v / withStats;
  return {
    matches: records.length,
    wins,
    losses,
    winRate: decided ? wins / decided : null,
    mvps,
    bestStreak,
    avg: withStats
      ? {
          score: per(sum.score),
          goals: per(sum.goals),
          assists: per(sum.assists),
          saves: per(sum.saves),
          shots: per(sum.shots),
        }
      : null,
  };
}

export interface Bucket {
  /** Local day (`YYYY-MM-DD`) or the Monday of the week. */
  key: string;
  start: Date;
  wins: number;
  losses: number;
  other: number;
}

function localDay(d: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
}

function startOfWeek(d: Date): Date {
  const day = startOfDay(d);
  const shift = (day.getDay() + 6) % 7; // Monday = 0
  return new Date(day.getFullYear(), day.getMonth(), day.getDate() - shift);
}

/**
 * Results per day (or per week past `maxDays` days of span), gaps filled
 * with empty buckets so the time axis stays regular.
 */
export function resultsOverTime(
  records: readonly MatchRecord[],
  maxDays = 45,
): { unit: "day" | "week"; buckets: Bucket[] } {
  if (records.length === 0) return { unit: "day", buckets: [] };
  const dates = records.map((r) => new Date(r.endedAt));
  const first = startOfDay(new Date(Math.min(...dates.map((d) => d.getTime()))));
  const last = startOfDay(new Date(Math.max(...dates.map((d) => d.getTime()))));
  const spanDays = Math.round((last.getTime() - first.getTime()) / 86_400_000) + 1;
  const unit = spanDays > maxDays ? "week" : "day";
  const floor = unit === "day" ? startOfDay : startOfWeek;
  const buckets = new Map<string, Bucket>();
  const step = (d: Date) =>
    unit === "day"
      ? new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1)
      : new Date(d.getFullYear(), d.getMonth(), d.getDate() + 7);
  for (let d = floor(first); d <= last; d = step(d)) {
    buckets.set(localDay(d), { key: localDay(d), start: d, wins: 0, losses: 0, other: 0 });
  }
  for (const [i, r] of records.entries()) {
    const date = dates[i];
    if (!date) continue;
    const b = buckets.get(localDay(floor(date)));
    if (!b) continue;
    if (r.result === "win") b.wins++;
    else if (r.result === "loss") b.losses++;
    else b.other++;
  }
  return { unit, buckets: [...buckets.values()] };
}

export interface FormPoint {
  index: number;
  endedAt: string;
  value: number;
  /** Average of the last `window` matches (fewer at the start). */
  average: number;
}

/** Score of each match and its rolling average (matches with your stats only). */
export function rollingScore(records: readonly MatchRecord[], window = 10): FormPoint[] {
  const out: FormPoint[] = [];
  const values: number[] = [];
  for (const r of records) {
    if (!r.you) continue;
    values.push(r.you.score);
    const recent = values.slice(-window);
    out.push({
      index: out.length,
      endedAt: r.endedAt,
      value: r.you.score,
      average: recent.reduce((a, b) => a + b, 0) / recent.length,
    });
  }
  return out;
}

export interface Group {
  key: string;
  matches: number;
  wins: number;
  losses: number;
  winRate: number | null;
}

function grouped(records: readonly MatchRecord[], keyOf: (r: MatchRecord) => string): Group[] {
  const map = new Map<string, Group>();
  for (const r of records) {
    const key = keyOf(r);
    const g = map.get(key) ?? { key, matches: 0, wins: 0, losses: 0, winRate: null };
    g.matches++;
    if (r.result === "win") g.wins++;
    if (r.result === "loss") g.losses++;
    map.set(key, g);
  }
  for (const g of map.values()) {
    const decided = g.wins + g.losses;
    g.winRate = decided ? g.wins / decided : null;
  }
  return [...map.values()];
}

/** Results per arena, most played first. */
export function byArena(records: readonly MatchRecord[]): Group[] {
  return grouped(records, (r) => r.arena ?? "").sort((a, b) => b.matches - a.matches);
}

export const DAY_PARTS = ["night", "morning", "afternoon", "evening"] as const;
export type DayPart = (typeof DAY_PARTS)[number];

/** 0–6 night, 6–12 morning, 12–18 afternoon, 18–24 evening (local time). */
export function dayPart(date: Date): DayPart {
  return DAY_PARTS[Math.floor(date.getHours() / 6)] ?? "night";
}

/** Results per part of the day, in the order of the day. */
export function byDayPart(records: readonly MatchRecord[]): Group[] {
  const groups = grouped(records, (r) => dayPart(new Date(r.endedAt)));
  return DAY_PARTS.flatMap((p) => groups.filter((g) => g.key === p));
}

export interface Session {
  start: string;
  end: string;
  matches: number;
  wins: number;
  losses: number;
}

/** Matches less than `gapMinutes` apart form a session. */
export function sessionsOf(records: readonly MatchRecord[], gapMinutes = 45): Session[] {
  const out: Session[] = [];
  let current: Session | null = null;
  let lastEnd = 0;
  for (const r of records) {
    const end = Date.parse(r.endedAt);
    if (!current || end - lastEnd > gapMinutes * 60_000) {
      current = { start: r.startedAt ?? r.endedAt, end: r.endedAt, matches: 0, wins: 0, losses: 0 };
      out.push(current);
    }
    current.end = r.endedAt;
    current.matches++;
    if (r.result === "win") current.wins++;
    if (r.result === "loss") current.losses++;
    lastEnd = end;
  }
  return out;
}

/** Best sessions: most wins, then best record. */
export function bestSessions(records: readonly MatchRecord[], count = 3): Session[] {
  return sessionsOf(records)
    .filter((s) => s.wins > 0)
    .sort((a, b) => b.wins - a.wins || a.losses - b.losses)
    .slice(0, count);
}
