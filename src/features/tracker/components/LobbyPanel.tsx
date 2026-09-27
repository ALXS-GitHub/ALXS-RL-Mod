import { LoaderCircle, Users } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import type { LobbyRating } from "../api";

function teamAverage(players: LobbyRating[]): number | null {
  const rated = players.filter((p) => p.rating != null);
  if (rated.length === 0) return null;
  return Math.round(rated.reduce((sum, p) => sum + (p.rating ?? 0), 0) / rated.length);
}

function PlayerRow({ p }: { p: LobbyRating }) {
  const { t } = useTranslation("tracker");
  return (
    <li className="flex h-8 items-center gap-2.5 text-[13px]">
      {p.tierIcon ? (
        <img src={p.tierIcon} alt="" className="size-5 shrink-0 object-contain" />
      ) : (
        <span className="size-5 shrink-0 rounded-full bg-white/[0.06]" aria-hidden />
      )}
      <span className="min-w-0 flex-1 truncate">{p.name}</span>
      {p.status === "pending" ? (
        <LoaderCircle className="size-3.5 animate-spin text-fg-subtle" aria-label={t("lobby.loading")} />
      ) : p.status === "ok" ? (
        <span
          className="text-right tabular-nums"
          title={[p.tier, p.division, p.playlist].filter(Boolean).join(" · ")}
        >
          {p.rating}
        </span>
      ) : (
        <span className="text-xs text-fg-subtle">{t("lobby.unavailable")}</span>
      )}
    </li>
  );
}

function TeamColumn({ team, players }: { team: 0 | 1; players: LobbyRating[] }) {
  const { t } = useTranslation("tracker");
  const avg = teamAverage(players);
  return (
    <div className="min-w-0 flex-1">
      <div className="mb-1 flex items-center justify-between text-xs">
        <span className={cn("font-medium", team === 0 ? "text-[#9cc1ff]" : "text-[#ffbe8f]")}>
          {team === 0 ? t("live.blue") : t("live.orange")}
        </span>
        <span className="text-fg-subtle tabular-nums">
          {avg != null ? t("lobby.average", { value: avg }) : "—"}
        </span>
      </div>
      <ul className="divide-y divide-line">
        {players.map((p) => (
          <PlayerRow key={p.key} p={p} />
        ))}
      </ul>
    </div>
  );
}

/** MMR of the players of the current match (Stats API ids → tracker.gg). */
export function LobbyPanel({ lobby }: { lobby: LobbyRating[] }) {
  const { t } = useTranslation("tracker");
  if (lobby.length === 0) return null;
  const blue = lobby.filter((p) => p.team === 0);
  const orange = lobby.filter((p) => p.team === 1);
  return (
    <div className="rounded-lg border border-line bg-white/[0.02] p-3">
      <p className="mb-2 flex items-center gap-1.5 text-xs text-fg-subtle">
        <Users className="size-3.5" />
        {t("lobby.title")}
      </p>
      <div className="flex gap-6">
        <TeamColumn team={0} players={blue} />
        <TeamColumn team={1} players={orange} />
      </div>
      <p className="mt-2 text-[11px] text-fg-subtle">{t("lobby.source")}</p>
    </div>
  );
}

export { teamAverage };
