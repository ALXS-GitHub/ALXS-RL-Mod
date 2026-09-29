import { TrendingUp } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Stat } from "@/components/ui/layout";
import { topMmrDelta } from "../api";
import { useRatings } from "../queries";
import { LobbyPanel } from "./LobbyPanel";

/** Session tab of the personal edition: your MMR delta and the lobby ranks. */
export function SessionRatings() {
  const { t } = useTranslation("personal");
  const { data: r } = useRatings();
  const mmr = r ? topMmrDelta(r.mmr) : null;
  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-4 rounded-lg border border-line bg-white/[0.02] px-3.5 py-2.5">
        <TrendingUp className="size-4 shrink-0 text-fg-subtle" />
        <Stat
          label={t("stats.mmr")}
          value={mmr ? `${mmr.delta >= 0 ? "+" : ""}${mmr.delta}` : "—"}
          tone={mmr ? (mmr.delta >= 0 ? "success" : "danger") : "default"}
          hint={mmr ? `${mmr.playlist} · ${mmr.latest}` : t("stats.mmrHint")}
        />
      </div>
      <LobbyPanel lobby={r?.lobby ?? []} />
    </div>
  );
}
