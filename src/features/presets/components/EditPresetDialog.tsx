import { ArrowRight, Brush, Camera, Map as MapIcon, Palette, X } from "lucide-react";
import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Tooltip } from "@/components/ui/tooltip";
import type { CatalogItem } from "@/features/items/api";
import { ItemThumb } from "@/features/items/components/ItemThumb";
import { itemLabel } from "@/features/items/constants";
import { useMaps } from "@/features/maps/queries";
import { type Preset, paletteSwatches, payloadName } from "../api";
import { useSavePreset, useUpdatePresetFromCurrent } from "../queries";

interface EditPresetDialogProps {
  preset: Preset | null;
  byId: Map<number, CatalogItem>;
  onClose: () => void;
}

function PartRow({
  lead,
  title,
  detail,
  onRemove,
}: {
  lead: ReactNode;
  title: string;
  detail?: string;
  onRemove: () => void;
}) {
  const { t } = useTranslation("presets");
  return (
    <li className="flex items-center gap-3 rounded-[8px] border border-line bg-white/[0.02] p-2">
      <div className="grid size-10 shrink-0 place-items-center overflow-hidden rounded-[6px] bg-white/[0.04]">
        {lead}
      </div>
      <div className="min-w-0 flex-1">
        <p className="truncate text-[13px] font-medium">{title}</p>
        {detail ? <p className="truncate text-[11px] text-fg-muted">{detail}</p> : null}
      </div>
      <Tooltip content={t("edit.remove")}>
        <Button size="icon-sm" variant="ghost" aria-label={t("edit.remove")} onClick={onRemove}>
          <X />
        </Button>
      </Tooltip>
    </li>
  );
}

/** Rename a preset, drop some of its parts, or replace it with the current setup. */
export function EditPresetDialog({ preset, byId, onClose }: EditPresetDialogProps) {
  const { t } = useTranslation(["presets", "items"]);
  const save = useSavePreset();
  const fromCurrent = useUpdatePresetFromCurrent();
  const maps = useMaps();
  const [draft, setDraft] = useState<Preset | null>(preset);
  useEffect(() => setDraft(preset), [preset]);

  if (!draft) return null;
  const valid = draft.name.trim().length > 0;
  const mapName = maps.data?.find((m) => m.id === draft.mapId)?.name;
  const swatches = paletteSwatches(draft.palette, 3);
  const empty = draft.swaps.length === 0 && !draft.palette && !draft.decal && !draft.mapId;

  return (
    <Dialog open={preset !== null} onOpenChange={(o) => !o && onClose()}>
      <DialogContent
        size="md"
        title={t("edit.title")}
        description={t("edit.description")}
        footer={
          <>
            <Tooltip content={t("edit.fromCurrentHint")}>
              <Button
                variant="ghost"
                className="mr-auto"
                loading={fromCurrent.isPending}
                onClick={() => fromCurrent.mutate(draft.id, { onSuccess: onClose })}
              >
                <Camera />
                {t("edit.fromCurrent")}
              </Button>
            </Tooltip>
            <Button variant="ghost" onClick={onClose}>
              {t("common:actions.cancel")}
            </Button>
            <Button
              variant="primary"
              disabled={!valid}
              loading={save.isPending}
              onClick={() => save.mutate({ ...draft, name: draft.name.trim() }, { onSuccess: onClose })}
            >
              {t("common:actions.save")}
            </Button>
          </>
        }
      >
        <div className="flex flex-col gap-4">
          <label className="flex flex-col gap-1.5 text-xs text-fg-muted">
            {t("nameLabel")}
            <Input
              maxLength={60}
              value={draft.name}
              onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            />
          </label>

          {empty ? (
            <p className="text-[13px] text-fg-muted">{t("edit.empty")}</p>
          ) : (
            <ul className="flex max-h-[50vh] flex-col gap-1.5 overflow-y-auto">
              {draft.swaps.map((s, i) => {
                const wanted = byId.get(s.wantedId);
                const owned = byId.get(s.ownedId);
                return (
                  <PartRow
                    key={`${s.slot}-${s.ownedId}-${s.wantedId}`}
                    lead={<ItemThumb item={wanted} className="size-10" />}
                    title={wanted ? itemLabel(wanted) : `#${s.wantedId}`}
                    detail={`${t(`items:slots.${s.slot}`)} · ${t("edit.replaces", {
                      name: owned ? itemLabel(owned) : `#${s.ownedId}`,
                    })}`}
                    onRemove={() => setDraft({ ...draft, swaps: draft.swaps.filter((_, j) => j !== i) })}
                  />
                );
              })}
              {draft.palette ? (
                <PartRow
                  lead={
                    swatches.length > 0 ? (
                      <span className="flex -space-x-1.5">
                        {swatches.map((c) => (
                          <span
                            key={c}
                            className="size-4 rounded-full border border-black/40"
                            style={{ background: c }}
                          />
                        ))}
                      </span>
                    ) : (
                      <Palette className="size-4 text-fg-muted" />
                    )
                  }
                  title={payloadName(draft.palette) ?? t("parts.palette")}
                  detail={t("parts.palette")}
                  onRemove={() => setDraft({ ...draft, palette: null })}
                />
              ) : null}
              {draft.decal ? (
                <PartRow
                  lead={<Brush className="size-4 text-fg-muted" />}
                  title={payloadName(draft.decal) ?? t("parts.decal")}
                  detail={t("parts.decal")}
                  onRemove={() => setDraft({ ...draft, decal: null })}
                />
              ) : null}
              {draft.mapId ? (
                <PartRow
                  lead={<MapIcon className="size-4 text-fg-muted" />}
                  title={mapName ?? t("parts.map")}
                  detail={t("parts.map")}
                  onRemove={() => setDraft({ ...draft, mapId: null })}
                />
              ) : null}
            </ul>
          )}

          <p className="flex items-start gap-2 text-xs text-fg-subtle">
            <ArrowRight className="mt-0.5 size-3.5 shrink-0" />
            {t("edit.workflow")}
          </p>
        </div>
      </DialogContent>
    </Dialog>
  );
}
