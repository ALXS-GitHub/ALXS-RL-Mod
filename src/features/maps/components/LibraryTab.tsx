import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { FolderInput, Map as MapIcon, Star } from "lucide-react";
import { motion } from "motion/react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { EmptyState, ErrorState, Skeleton } from "@/components/ui/feedback";
import { SearchInput } from "@/components/ui/input";
import { Segmented } from "@/components/ui/segmented";
import { notify } from "@/components/ui/toast";
import { useGameStatus } from "@/lib/game";
import type { MapEntry } from "../api";
import {
  useActivateMap,
  useDeleteMap,
  useMapSession,
  useMaps,
  usePlayOffline,
  useUpdateMap,
} from "../queries";
import { EditMapDialog } from "./EditMapDialog";
import { MapCard } from "./MapCard";

type Filter = "all" | "favorites";

export function LibraryTab({ onImport }: { onImport: () => void }) {
  const { t } = useTranslation("maps");
  const { t: tc } = useTranslation();
  const maps = useMaps();
  const { data: session } = useMapSession();
  const { data: game } = useGameStatus();
  const activate = useActivateMap();
  const playOffline = usePlayOffline();
  const update = useUpdateMap();
  const remove = useDeleteMap();
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [editing, setEditing] = useState<MapEntry | null>(null);
  const [deleting, setDeleting] = useState<MapEntry | null>(null);

  const gameRunning = game?.running.running ?? false;
  const busyId = activate.isPending
    ? activate.variables?.id
    : playOffline.isPending
      ? playOffline.variables
      : undefined;

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (maps.data ?? []).filter(
      (m) =>
        (filter === "all" || m.favorite) &&
        (!q ||
          m.name.toLowerCase().includes(q) ||
          m.author?.toLowerCase().includes(q) ||
          m.tags.some((tag) => tag.toLowerCase().includes(q))),
    );
  }, [maps.data, query, filter]);

  if (maps.isError) return <ErrorState error={maps.error} onRetry={() => void maps.refetch()} />;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <SearchInput
          value={query}
          onValueChange={setQuery}
          placeholder={t("library.search")}
          className="w-full max-w-sm"
        />
        <Segmented
          value={filter}
          onValueChange={setFilter}
          size="sm"
          options={[
            { value: "all", label: t("library.all", { count: maps.data?.length ?? 0 }) },
            { value: "favorites", label: t("library.favorites"), icon: <Star /> },
          ]}
        />
        <span className="ml-auto text-xs text-fg-subtle">{t("library.dropHint")}</span>
      </div>

      {maps.isPending ? (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-4">
          {Array.from({ length: 8 }, (_, i) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: static placeholders
            <Skeleton key={i} className="aspect-[4/3.4] rounded-lg" />
          ))}
        </div>
      ) : visible.length === 0 ? (
        <EmptyState
          icon={filter === "favorites" ? Star : MapIcon}
          title={maps.data?.length ? t("library.noMatch") : t("library.emptyTitle")}
          description={maps.data?.length ? undefined : t("library.emptyDescription")}
          action={
            maps.data?.length ? undefined : (
              <Button variant="primary" onClick={onImport}>
                <FolderInput />
                {t("actions.import")}
              </Button>
            )
          }
        />
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-4">
          {visible.map((map, i) => (
            <motion.div
              key={map.id}
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: Math.min(i, 12) * 0.035, type: "spring", stiffness: 300, damping: 30 }}
            >
              <MapCard
                map={map}
                active={session?.mapId === map.id}
                activeTarget={session?.mapId === map.id ? session.target : undefined}
                gameRunning={gameRunning}
                busy={busyId === map.id}
                onPlay={() =>
                  playOffline.mutate(map.id, {
                    onSuccess: () =>
                      notify.success(t("toast.launched", { name: map.name }), t("toast.launchedHint")),
                  })
                }
                onInstall={() =>
                  activate.mutate(
                    { id: map.id, target: null },
                    {
                      onSuccess: (s) =>
                        gameRunning
                          ? notify.success(t("toast.installedLive", { name: s.mapName }), t("toast.liveHint"))
                          : notify.success(t("toast.installed", { name: s.mapName })),
                    },
                  )
                }
                onFavorite={() => update.mutate({ id: map.id, patch: { favorite: !map.favorite } })}
                onEdit={() => setEditing(map)}
                onDelete={() => setDeleting(map)}
                onReveal={() => void revealItemInDir(`${map.folder}\\${map.fileName}`).catch(notify.error)}
              />
            </motion.div>
          ))}
        </div>
      )}

      <EditMapDialog map={editing} onClose={() => setEditing(null)} />

      <Dialog open={deleting !== null} onOpenChange={(open) => !open && setDeleting(null)}>
        <DialogContent
          size="sm"
          title={t("delete.title")}
          description={t("delete.description", { name: deleting?.name ?? "" })}
          footer={
            <>
              <Button variant="ghost" onClick={() => setDeleting(null)}>
                {tc("actions.cancel")}
              </Button>
              <Button
                variant="danger"
                loading={remove.isPending}
                onClick={() => deleting && remove.mutate(deleting.id, { onSuccess: () => setDeleting(null) })}
              >
                {tc("actions.delete")}
              </Button>
            </>
          }
        >
          <p className="text-sm text-fg-muted">{t("delete.warning")}</p>
        </DialogContent>
      </Dialog>
    </div>
  );
}
