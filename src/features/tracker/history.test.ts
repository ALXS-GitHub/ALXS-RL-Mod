import { describe, expect, it } from "vitest";
import type { MatchRecord } from "./api";
import {
  bestSessions,
  byArena,
  byDayPart,
  filterMatches,
  playlistsOf,
  resultsOverTime,
  rollingScore,
  summarize,
} from "./history";

const me = {
  name: "Me",
  team: 0,
  score: 400,
  goals: 1,
  assists: 1,
  saves: 2,
  shots: 3,
  demos: 0,
  touches: 20,
};

function rec(endedAt: string, result: MatchRecord["result"], extra: Partial<MatchRecord> = {}): MatchRecord {
  return {
    guid: null,
    arena: "Stadium_P",
    endedAt,
    result,
    yourTeam: 0,
    blueScore: 0,
    orangeScore: 0,
    overtime: false,
    you: {
      name: "Me",
      team: 0,
      score: 400,
      goals: 1,
      assists: 1,
      saves: 2,
      shots: 3,
      demos: 0,
      touches: 20,
    },
    mvp: false,
    ...extra,
  };
}

const at = (day: number, hour: number, minute = 0) => new Date(2026, 8, day, hour, minute).toISOString();

describe("match history", () => {
  const records = [
    rec(at(20, 21), "win", { playlistId: 11, mvp: true }),
    rec(at(20, 21, 10), "win", { playlistId: 11 }),
    rec(at(20, 21, 20), "loss", { playlistId: 13, arena: "Mannfield_P" }),
    rec(at(22, 9), "win", { playlistId: 11 }),
    rec(at(22, 9, 8), "unknown", { you: null }),
  ];

  it("summarizes wins, streaks and averages", () => {
    const s = summarize(records);
    expect([s.matches, s.wins, s.losses, s.mvps, s.bestStreak]).toEqual([5, 3, 1, 1, 2]);
    expect(s.winRate).toBeCloseTo(0.75);
    expect(s.avg?.saves).toBe(2);
    expect(summarize([]).avg).toBeNull();
  });

  it("filters by period and playlist", () => {
    const now = new Date(2026, 8, 22, 12);
    expect(filterMatches(records, { period: "7d", playlist: 11 }, now)).toHaveLength(3);
    expect(filterMatches(records, { period: "all", playlist: null }, now)).toHaveLength(5);
    expect(filterMatches(records, { period: "7d", playlist: null }, new Date(2026, 9, 30))).toHaveLength(0);
    expect(playlistsOf(records)).toEqual([11, 13]);
  });

  it("fills empty days", () => {
    const { unit, buckets } = resultsOverTime(records);
    expect(unit).toBe("day");
    expect(buckets.map((b) => [b.wins, b.losses, b.other])).toEqual([
      [2, 1, 0],
      [0, 0, 0],
      [1, 0, 1],
    ]);
  });

  it("switches to weeks on long spans", () => {
    const long = [rec(at(1, 12), "win"), rec(new Date(2026, 11, 1, 12).toISOString(), "loss")];
    const { unit, buckets } = resultsOverTime(long);
    expect(unit).toBe("week");
    expect(buckets.reduce((n, b) => n + b.wins + b.losses, 0)).toBe(2);
  });

  it("averages the score over a rolling window", () => {
    const scores = [100, 200, 300].map((score, i) => rec(at(20, 10 + i), "win", { you: { ...me, score } }));
    expect(rollingScore(scores, 2).map((p) => p.average)).toEqual([100, 150, 250]);
  });

  it("groups by arena, part of the day and session", () => {
    expect(byArena(records).map((g) => [g.key, g.matches])).toEqual([
      ["Stadium_P", 4],
      ["Mannfield_P", 1],
    ]);
    expect(byDayPart(records).map((g) => g.key)).toEqual(["morning", "evening"]);
    const best = bestSessions(records);
    expect(best.map((s) => [s.matches, s.wins, s.losses])).toEqual([
      [3, 2, 1],
      [2, 1, 0],
    ]);
  });
});
