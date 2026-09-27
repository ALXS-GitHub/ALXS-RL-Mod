import { openUrl } from "@tauri-apps/plugin-opener";
import { ChevronLeft, ChevronRight, CloudDownload, ExternalLink, Globe, Library } from "lucide-react";
import { motion } from "motion/react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { EmptyState, ErrorState, Skeleton } from "@/components/ui/feedback";
import { GlassCard } from "@/components/ui/glass";
import { SearchInput } from "@/components/ui/input";
import { Segmented } from "@/components/ui/segmented";
import { notify } from "@/components/ui/toast";
import { formatBytes } from "@/lib/format";
import type { DownloadProgress, RemoteMap, RemoteSource } from "../api";
import { useBrowse, useDownloadMap, useDownloadProgress } from "../queries";
import { MapArt } from "./MapArt";

function useDebounced<T>(value: T, delay = 350): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setV(value), delay);
    return () => clearTimeout(id);
  }, [value, delay]);
  return v;
}

function ProgressBar({ progress }: { progress: DownloadProgress }) {
  const { t } = useTranslation("maps");
  const pct = progress.total ? Math.min(100, (progress.downloaded / progress.total) * 100) : null;
  return (
    <div className="flex flex-col gap-1.5">
      <div className="h-1 overflow-hidden rounded-full bg-white/10">
        <motion.div
          className="h-full rounded-full bg-[var(--color-accent)]"
          animate={{ width: pct === null ? "35%" : `${pct}%` }}
          transition={{ ease: "easeOut" }}
        />
      </div>
      <p className="text-[11px] text-fg-subtle tabular-nums">
        {t(`browse.phase.${progress.phase}`)}
        {progress.phase === "download"
          ? ` · ${formatBytes(progress.downloaded)}${progress.total ? ` / ${formatBytes(progress.total)}` : ""}`
          : ""}
      </p>
    </div>
  );
}

interface RemoteCardProps {
  map: RemoteMap;
  progress: DownloadProgress | undefined;
  downloading: boolean;
  onDownload: () => void;
  onOpenLibrary: () => void;
}

function RemoteCard({ map, progress, downloading, onDownload, onOpenLibrary }: RemoteCardProps) {
  const { t } = useTranslation("maps");
  const openPage = () => void openUrl(map.pageUrl).catch(notify.error);
  return (
    <GlassCard className="group flex flex-col overflow-hidden">
      <div className="relative">
        <MapArt name={map.name} src={map.previewUrl} className="aspect-[16/9] w-full" />
        {map.installedMapId ? (
          <Badge tone="success" dot className="absolute top-2.5 left-2.5">
            {t("browse.inLibrary")}
          </Badge>
        ) : null}
        <div className="absolute inset-x-3 bottom-2.5">
          <p className="truncate text-[13px] font-semibold text-white">{map.name}</p>
          <p className="truncate text-xs text-white/65">
            {map.author ?? t("browse.unknownAuthor")}
            {map.sizeBytes ? ` · ${formatBytes(map.sizeBytes)}` : ""}
          </p>
        </div>
      </div>
      <div className="flex flex-1 flex-col gap-3 p-3">
        {map.description ? <p className="line-clamp-2 text-xs text-fg-muted">{map.description}</p> : null}
        {map.tags.length ? (
          <div className="flex flex-wrap gap-1">
            {map.tags.slice(0, 4).map((tag) => (
              <Badge key={tag}>{tag}</Badge>
            ))}
          </div>
        ) : null}
        <div className="mt-auto">
          {progress ? (
            <ProgressBar progress={progress} />
          ) : map.installedMapId ? (
            <Button size="sm" variant="secondary" className="w-full" onClick={onOpenLibrary}>
              <Library />
              {t("browse.showInLibrary")}
            </Button>
          ) : map.downloadable ? (
            <div className="flex gap-1.5">
              <Button
                size="sm"
                variant="primary"
                className="flex-1"
                onClick={onDownload}
                loading={downloading}
              >
                <CloudDownload />
                {t("browse.download")}
              </Button>
              <Button size="icon-sm" variant="ghost" onClick={openPage} aria-label={t("browse.openPage")}>
                <ExternalLink />
              </Button>
            </div>
          ) : (
            <div className="flex flex-col gap-1.5">
              <Button size="sm" variant="secondary" className="w-full" onClick={openPage}>
                <ExternalLink />
                {t("browse.openPage")}
              </Button>
              <p className="text-[11px] text-fg-subtle">{t("browse.externalHint")}</p>
            </div>
          )}
        </div>
      </div>
    </GlassCard>
  );
}

export function BrowseTab({ onShowLibrary }: { onShowLibrary: () => void }) {
  const { t } = useTranslation("maps");
  const [source, setSource] = useState<RemoteSource>("bakkesPlugins");
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(1);
  const debounced = useDebounced(query);
  const browse = useBrowse(source, debounced, page);
  const download = useDownloadMap();
  const progress = useDownloadProgress();

  // Back to page 1 whenever the search or the source changes.
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset on input change only
  useEffect(() => setPage(1), [debounced, source]);

  const data = browse.data;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <Segmented
          value={source}
          onValueChange={setSource}
          size="sm"
          options={[
            { value: "bakkesPlugins", label: "BakkesPlugins", icon: <Globe /> },
            { value: "lethamyr", label: "Lethamyr", icon: <Globe /> },
          ]}
        />
        <SearchInput
          value={query}
          onValueChange={setQuery}
          placeholder={source === "lethamyr" ? t("browse.searchPage") : t("browse.search")}
          className="w-full max-w-sm"
        />
        <div className="ml-auto flex items-center gap-2 text-xs text-fg-subtle">
          <Button
            size="icon-sm"
            variant="ghost"
            disabled={page <= 1 || browse.isFetching}
            onClick={() => setPage((p) => Math.max(1, p - 1))}
            aria-label={t("browse.previous")}
          >
            <ChevronLeft />
          </Button>
          <span className="tabular-nums">
            {data?.totalPages
              ? t("browse.pageOf", { page, total: data.totalPages })
              : t("browse.page", { page })}
          </span>
          <Button
            size="icon-sm"
            variant="ghost"
            disabled={!data?.hasNext || browse.isFetching}
            onClick={() => setPage((p) => p + 1)}
            aria-label={t("browse.next")}
          >
            <ChevronRight />
          </Button>
        </div>
      </div>

      {source === "lethamyr" ? (
        <p className="glass-inset rounded-[8px] px-3 py-2 text-xs text-fg-muted">
          {t("browse.lethamyrNote")}
        </p>
      ) : null}

      {browse.isError ? (
        <ErrorState error={browse.error} onRetry={() => void browse.refetch()} />
      ) : browse.isPending ? (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-4">
          {Array.from({ length: 8 }, (_, i) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: static placeholders
            <Skeleton key={i} className="aspect-[4/3.6] rounded-lg" />
          ))}
        </div>
      ) : !data?.items.length ? (
        <EmptyState icon={Globe} title={t("browse.empty")} />
      ) : (
        <div
          className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-4 transition-opacity"
          style={{ opacity: browse.isPlaceholderData ? 0.55 : 1 }}
        >
          {data.items.map((map, i) => (
            <motion.div
              key={`${map.source}:${map.remoteId}`}
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: Math.min(i, 12) * 0.03, type: "spring", stiffness: 300, damping: 30 }}
            >
              <RemoteCard
                map={map}
                progress={progress[`${map.source}:${map.remoteId}`]}
                downloading={download.isPending && download.variables?.remoteId === map.remoteId}
                onOpenLibrary={onShowLibrary}
                onDownload={() =>
                  download.mutate(
                    { source: map.source, remoteId: map.remoteId },
                    { onSuccess: (m) => notify.success(t("toast.downloaded", { name: m.name })) },
                  )
                }
              />
            </motion.div>
          ))}
        </div>
      )}
    </div>
  );
}
