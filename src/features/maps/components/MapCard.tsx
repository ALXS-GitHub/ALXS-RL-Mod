import { ArrowRightLeft, Check, FolderOpen, Gamepad2, Pencil, Repeat, Star, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { GlassCard } from "@/components/ui/glass";
import { Tooltip } from "@/components/ui/tooltip";
import { cn } from "@/lib/cn";
import { formatBytes, formatRelative } from "@/lib/format";
import { DEFAULT_TARGET, type MapEntry, targetLabel } from "../api";
import { localPreview, MapArt } from "./MapArt";

interface MapCardProps {
  map: MapEntry;
  active: boolean;
  /** Labs arena this map replaces right now (when active). */
  activeTarget?: string;
  gameRunning: boolean;
  busy: boolean;
  onPlay: () => void;
  onInstall: () => void;
  onFavorite: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onReveal: () => void;
}

function originLabelKey(map: MapEntry): string {
  switch (map.origin.kind) {
    case "remote":
      return map.origin.source === "bakkesPlugins" ? "origin.bakkesPlugins" : "origin.lethamyr";
    case "bakkesMod":
      return "origin.bakkesMod";
    case "other":
      return "origin.other";
    default:
      return "origin.imported";
  }
}

export function MapCard({
  map,
  active,
  activeTarget,
  gameRunning,
  busy,
  onPlay,
  onInstall,
  onFavorite,
  onEdit,
  onDelete,
  onReveal,
}: MapCardProps) {
  const { t } = useTranslation("maps");
  const src = localPreview(map.folder, map.previewFile) ?? map.previewUrl;

  return (
    <GlassCard selected={active} className="group flex flex-col overflow-hidden">
      <div className="relative">
        <MapArt name={map.name} src={src} className="aspect-[16/9] w-full" />
        <div className="absolute top-2.5 left-2.5 flex gap-1.5">
          {active ? (
            <Badge tone="accent" dot>
              {t("card.installed")}
            </Badge>
          ) : null}
          <Badge tone="neutral" className="bg-black/50">
            {t(originLabelKey(map))}
          </Badge>
        </div>
        <button
          type="button"
          onClick={onFavorite}
          aria-label={t("card.favorite")}
          aria-pressed={map.favorite}
          className={cn(
            "absolute top-2 right-2 grid size-7 place-items-center rounded-[6px] bg-black/45 transition-colors hover:bg-black/60",
            map.favorite ? "text-warning" : "text-white/70 opacity-0 group-hover:opacity-100",
          )}
        >
          <Star className={cn("size-3.5", map.favorite && "fill-current")} />
        </button>
        <div className="absolute inset-x-3 bottom-2.5">
          <p className="truncate text-[13px] font-semibold text-white">{map.name}</p>
          <p className="truncate text-xs text-white/65">
            {map.author ? `${map.author} · ` : ""}
            {formatBytes(map.sizeBytes)}
          </p>
        </div>
      </div>

      <div className="flex flex-1 flex-col gap-3 p-3">
        <div className="flex min-h-5 flex-wrap items-center gap-1.5 text-[11px] text-fg-subtle">
          <Badge tone="blue" title={t("card.replacesHint")}>
            <ArrowRightLeft />
            {targetLabel(activeTarget ?? map.preferredTarget ?? DEFAULT_TARGET)}
          </Badge>
          {map.tags.slice(0, 3).map((tag) => (
            <Badge key={tag}>{tag}</Badge>
          ))}
          <span className="ml-auto">
            {map.lastPlayedAt
              ? t("card.lastPlayed", { when: formatRelative(map.lastPlayedAt) })
              : t("card.added", { when: formatRelative(map.addedAt) })}
          </span>
        </div>

        <div className="mt-auto flex items-center gap-1.5">
          <Tooltip content={gameRunning ? t("card.installHintLive") : t("card.installHint")}>
            <span className="flex-1">
              <Button
                variant={active ? "secondary" : "primary"}
                size="sm"
                className="w-full"
                onClick={onInstall}
                loading={busy}
                disabled={busy || active}
              >
                {active ? <Check /> : <Repeat />}
                {active ? t("card.swapped") : t("card.swap")}
              </Button>
            </span>
          </Tooltip>
          <Tooltip content={gameRunning ? t("card.playOfflineRunning") : t("card.playOfflineHint")}>
            <span>
              <Button
                size="icon-sm"
                variant="secondary"
                aria-label={t("card.playOffline")}
                onClick={onPlay}
                disabled={busy || gameRunning}
              >
                <Gamepad2 />
              </Button>
            </span>
          </Tooltip>
          <Tooltip content={t("card.edit")}>
            <Button size="icon-sm" variant="ghost" onClick={onEdit}>
              <Pencil />
            </Button>
          </Tooltip>
          <Tooltip content={t("card.reveal")}>
            <Button size="icon-sm" variant="ghost" onClick={onReveal}>
              <FolderOpen />
            </Button>
          </Tooltip>
          <Tooltip content={t("card.delete")}>
            <Button
              size="icon-sm"
              variant="ghost"
              className="hover:text-danger"
              onClick={onDelete}
              disabled={active}
            >
              <Trash2 />
            </Button>
          </Tooltip>
        </div>
      </div>
    </GlassCard>
  );
}
