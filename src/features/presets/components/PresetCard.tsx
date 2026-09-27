import { Brush, Map as MapIcon, Palette, Pencil, Play, Share2, Sparkles, Trash2 } from "lucide-react";
import { motion } from "motion/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { GlassCard } from "@/components/ui/glass";
import { Tooltip } from "@/components/ui/tooltip";
import type { CatalogItem } from "@/features/items/api";
import { ItemThumb } from "@/features/items/components/ItemThumb";
import { formatRelative } from "@/lib/format";
import { type Preset, paletteSwatches, payloadName } from "../api";

interface PresetCardProps {
  preset: Preset;
  index: number;
  byId: Map<number, CatalogItem>;
  applying: boolean;
  /** Presets change item files: game closed only. */
  gameRunning: boolean;
  onApply: () => void;
  onShare: () => void;
  onEdit: () => void;
  onDelete: () => void;
}

export function PresetCard({
  preset,
  index,
  byId,
  applying,
  gameRunning,
  onApply,
  onShare,
  onEdit,
  onDelete,
}: PresetCardProps) {
  const { t } = useTranslation("presets");
  const [confirmDelete, setConfirmDelete] = useState(false);
  const wanted = preset.swaps.slice(0, 5).map((s) => byId.get(s.wantedId));
  const swatches = paletteSwatches(preset.palette);
  const paletteName = payloadName(preset.palette);
  const decalName = payloadName(preset.decal);

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 14, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, scale: 0.96 }}
      transition={{ delay: Math.min(index, 12) * 0.035, type: "spring", stiffness: 320, damping: 30 }}
    >
      <GlassCard className="group flex h-full flex-col gap-3 p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <h3 className="truncate text-sm font-semibold">{preset.name}</h3>
            <p className="text-[11px] text-fg-subtle">
              {t("card.updated", { when: formatRelative(preset.updatedAt) })}
            </p>
          </div>
          <div className="flex shrink-0 gap-1 opacity-60 transition-opacity group-hover:opacity-100">
            <Tooltip content={t("card.share")}>
              <Button size="icon-sm" variant="ghost" aria-label={t("card.share")} onClick={onShare}>
                <Share2 />
              </Button>
            </Tooltip>
            <Tooltip content={t("card.edit")}>
              <Button size="icon-sm" variant="ghost" aria-label={t("card.edit")} onClick={onEdit}>
                <Pencil />
              </Button>
            </Tooltip>
            <Tooltip content={confirmDelete ? t("card.confirmDelete") : t("card.delete")}>
              <Button
                size="icon-sm"
                variant={confirmDelete ? "danger" : "ghost"}
                aria-label={t("card.delete")}
                onBlur={() => setConfirmDelete(false)}
                onClick={() => (confirmDelete ? onDelete() : setConfirmDelete(true))}
              >
                <Trash2 />
              </Button>
            </Tooltip>
          </div>
        </div>

        {/* Stacked thumbnails of the wanted items. */}
        <div className="flex h-12 items-center gap-1.5">
          {wanted.length > 0 ? (
            wanted.map((item, i) => (
              <ItemThumb
                key={preset.swaps[i]?.wantedId ?? i}
                item={item}
                className="size-12 border border-line"
              />
            ))
          ) : (
            <p className="text-xs text-fg-subtle">{t("card.noSwaps")}</p>
          )}
        </div>

        <div className="flex flex-wrap gap-1.5">
          <Badge tone="accent">
            <Sparkles />
            {t("card.swaps", { count: preset.swaps.length })}
          </Badge>
          {preset.palette ? (
            <Badge tone="neutral">
              <Palette />
              {paletteName ?? t("parts.palette")}
              {swatches.length > 0 ? (
                <span className="ml-0.5 flex -space-x-1">
                  {swatches.map((c) => (
                    <span
                      key={c}
                      className="size-2.5 rounded-full border border-black/40"
                      style={{ background: c }}
                    />
                  ))}
                </span>
              ) : null}
            </Badge>
          ) : null}
          {preset.decal ? (
            <Badge tone="neutral">
              <Brush />
              {decalName ?? t("parts.decal")}
            </Badge>
          ) : null}
          {preset.mapId ? (
            <Badge tone="neutral">
              <MapIcon />
              {t("parts.map")}
            </Badge>
          ) : null}
        </div>

        <Button
          variant="primary"
          className="mt-auto w-full"
          loading={applying}
          disabled={gameRunning}
          title={gameRunning ? t("card.closeGame") : undefined}
          onClick={onApply}
        >
          <Play />
          {t("card.apply")}
        </Button>
      </GlassCard>
    </motion.div>
  );
}
