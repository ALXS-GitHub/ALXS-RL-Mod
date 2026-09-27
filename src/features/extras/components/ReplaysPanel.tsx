import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Clapperboard, FolderOpen, RefreshCw } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { EmptyState, ErrorState, Skeleton } from "@/components/ui/feedback";
import { GlassPanel } from "@/components/ui/glass";
import { SearchInput } from "@/components/ui/input";
import { notify } from "@/components/ui/toast";
import { Tooltip } from "@/components/ui/tooltip";
import { arenaLabel, formatClock } from "@/features/tracker/api";
import { formatBytes, formatDateTime } from "@/lib/format";
import type { ReplayFile } from "../api";
import { useReplays } from "../queries";

const PAGE = 60;

function title(r: ReplayFile): string {
  return r.header?.name || r.fileName.replace(/\.replay$/i, "");
}

export function ReplaysPanel() {
  const { t } = useTranslation("extras");
  const replays = useReplays();
  const [query, setQuery] = useState("");
  const [limit, setLimit] = useState(PAGE);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = replays.data?.replays ?? [];
    if (!q) return list;
    return list.filter(
      (r) =>
        title(r).toLowerCase().includes(q) ||
        r.header?.map?.toLowerCase().includes(q) ||
        r.header?.playerName?.toLowerCase().includes(q),
    );
  }, [replays.data, query]);

  const reveal = (path: string) => void revealItemInDir(path).catch(notify.error);
  const folder = replays.data?.folders[0];

  return (
    <GlassPanel className="flex flex-col gap-3 p-4">
      <div className="flex flex-wrap items-center gap-3">
        <div className="flex items-center gap-2.5">
          <Clapperboard className="size-4 text-fg-subtle" />
          <h2 className="text-sm font-semibold">{t("replays.title")}</h2>
          {replays.data ? (
            <span className="text-xs text-fg-subtle">
              {t("replays.count", { count: replays.data.replays.length })}
            </span>
          ) : null}
        </div>
        <SearchInput
          value={query}
          onValueChange={setQuery}
          placeholder={t("replays.search")}
          className="ml-auto w-64"
        />
        <Button
          size="icon-sm"
          variant="ghost"
          onClick={() => void replays.refetch()}
          aria-label={t("replays.refresh")}
        >
          <RefreshCw className={replays.isFetching ? "animate-spin" : undefined} />
        </Button>
        {folder ? (
          <Button size="sm" variant="secondary" onClick={() => reveal(folder)}>
            <FolderOpen />
            {t("replays.openFolder")}
          </Button>
        ) : null}
      </div>

      {replays.isError ? (
        <ErrorState error={replays.error} onRetry={() => void replays.refetch()} />
      ) : replays.isPending ? (
        <Skeleton className="h-72" />
      ) : !filtered.length ? (
        <EmptyState
          icon={Clapperboard}
          title={replays.data?.replays.length ? t("replays.noMatch") : t("replays.empty")}
          description={replays.data?.replays.length ? undefined : t("replays.emptyHint")}
        />
      ) : (
        <div className="overflow-x-auto rounded-[8px] border border-line">
          <table className="w-full text-[13px]">
            <thead className="bg-white/[0.03] text-left text-xs text-fg-subtle">
              <tr>
                <th className="px-3 py-2 font-medium">{t("replays.cols.name")}</th>
                <th className="px-3 py-2 font-medium">{t("replays.cols.map")}</th>
                <th className="px-3 py-2 text-center font-medium">{t("replays.cols.score")}</th>
                <th className="px-3 py-2 text-right font-medium">{t("replays.cols.duration")}</th>
                <th className="px-3 py-2 text-right font-medium">{t("replays.cols.date")}</th>
                <th className="px-3 py-2 text-right font-medium">{t("replays.cols.size")}</th>
                <th className="w-10" />
              </tr>
            </thead>
            <tbody>
              {filtered.slice(0, limit).map((r) => {
                const h = r.header;
                return (
                  <tr
                    key={r.path}
                    className="group border-line border-t transition-colors hover:bg-white/[0.03]"
                  >
                    <td className="max-w-[240px] truncate px-3 py-2 font-medium">
                      {title(r)}
                      {h?.teamSize ? (
                        <span className="ml-2 text-xs text-fg-subtle">
                          {h.teamSize}v{h.teamSize}
                        </span>
                      ) : null}
                    </td>
                    <td className="px-3 py-2 text-fg-muted">{arenaLabel(h?.map)}</td>
                    <td className="px-3 py-2 text-center tabular-nums">
                      <span className="text-[#9cc1ff]">{h?.blueScore ?? 0}</span>
                      <span className="mx-1 text-fg-subtle">-</span>
                      <span className="text-[#ffbe8f]">{h?.orangeScore ?? 0}</span>
                    </td>
                    <td className="px-3 py-2 text-right text-fg-muted tabular-nums">
                      {formatClock(h?.durationSeconds)}
                    </td>
                    <td className="whitespace-nowrap px-3 py-2 text-right text-fg-muted tabular-nums">
                      {r.modifiedAt ? formatDateTime(r.modifiedAt) : "—"}
                    </td>
                    <td className="px-3 py-2 text-right text-fg-subtle tabular-nums">
                      {formatBytes(r.sizeBytes)}
                    </td>
                    <td className="px-2 py-1">
                      <Tooltip content={t("replays.reveal")}>
                        <Button
                          size="icon-sm"
                          variant="ghost"
                          className="opacity-0 group-hover:opacity-100"
                          onClick={() => reveal(r.path)}
                        >
                          <FolderOpen />
                        </Button>
                      </Tooltip>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          {filtered.length > limit ? (
            <div className="border-line border-t p-2 text-center">
              <Button size="sm" variant="ghost" onClick={() => setLimit((l) => l + PAGE)}>
                {t("replays.more", { count: filtered.length - limit })}
              </Button>
            </div>
          ) : null}
        </div>
      )}
    </GlassPanel>
  );
}
