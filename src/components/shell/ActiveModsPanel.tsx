import { useNavigate } from "@tanstack/react-router";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  Brush,
  Layers,
  type LucideIcon,
  Map as MapIcon,
  Palette as PaletteIcon,
  PanelRightClose,
  PanelRightOpen,
  Pencil,
  RotateCcw,
  Save,
  Sparkles,
  Undo2,
  Volleyball,
} from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { notify } from "@/components/ui/toast";
import { Tooltip } from "@/components/ui/tooltip";
import { useBallStatus, useRemoveBall } from "@/features/ball/queries";
import { useDecalStatus, useRemoveDecal } from "@/features/decals/queries";
import { ItemThumb } from "@/features/items/components/ItemThumb";
import { SwapColorBadge } from "@/features/items/components/SwapColorBadge";
import { SLOT_ICONS } from "@/features/items/constants";
import { useCatalog, useRestoreSwap, useSwaps } from "@/features/items/queries";
import { targetLabel } from "@/features/maps/api";
import { localPreview, MapArt } from "@/features/maps/components/MapArt";
import { useDeactivateMap, useMapSession, useMaps } from "@/features/maps/queries";
import type { Rgb } from "@/features/palette/api";
import { usePaletteStatus, usePalettes, useRestorePalette } from "@/features/palette/queries";
import { useActivePreset, useUpdatePresetFromCurrent } from "@/features/presets/queries";
import { RestoreStockDialog } from "@/features/settings/components/RestoreStock";
import { formatRelative } from "@/lib/format";
import { useGameStatus } from "@/lib/game";
import { useUi } from "@/stores/ui";

const OPEN_WIDTH = 300;
const RAIL_WIDTH = 44;

const rgb = (c: Rgb | undefined) => (c ? `rgb(${c.r} ${c.g} ${c.b})` : "transparent");

interface RowProps {
  lead: ReactNode;
  kicker: ReactNode;
  title: string;
  detail?: ReactNode;
  restoreLabel: string;
  /** Why restoring is not possible right now (tooltip). */
  blockedReason?: string;
  restoring: boolean;
  onRestore: () => void;
  /** Extra actions shown before restore (edit, update…). */
  actions?: RowAction[];
}

interface RowAction {
  key: string;
  icon: ReactNode;
  label: string;
  onClick: () => void;
  loading?: boolean;
  disabled?: boolean;
}

function ModRow({
  lead,
  kicker,
  title,
  detail,
  restoreLabel,
  blockedReason,
  restoring,
  onRestore,
  actions = [],
}: RowProps) {
  return (
    <motion.li
      layout
      initial={{ opacity: 0, x: 12 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, height: 0 }}
      className="group flex items-center gap-2.5 rounded-md px-2 py-1.5 transition-colors hover:bg-white/[0.04]"
    >
      <div className="grid size-9 shrink-0 place-items-center overflow-hidden rounded-[6px] bg-white/[0.04]">
        {lead}
      </div>
      <div className="min-w-0 flex-1">
        <p className="flex items-center gap-1.5 text-[11px] text-fg-subtle">{kicker}</p>
        <p className="truncate text-[13px] font-medium" title={title}>
          {title}
        </p>
        {detail ? <p className="truncate text-[11px] text-fg-muted">{detail}</p> : null}
      </div>
      {actions.map((a) => (
        <Tooltip key={a.key} content={a.label}>
          <span>
            <Button
              size="icon-sm"
              variant="ghost"
              aria-label={a.label}
              className="opacity-60 group-hover:opacity-100"
              disabled={a.disabled}
              loading={a.loading}
              onClick={a.onClick}
            >
              {a.icon}
            </Button>
          </span>
        </Tooltip>
      ))}
      <Tooltip content={blockedReason ?? restoreLabel}>
        <span>
          <Button
            size="icon-sm"
            variant="ghost"
            aria-label={restoreLabel}
            className="opacity-60 group-hover:opacity-100"
            disabled={Boolean(blockedReason)}
            loading={restoring}
            onClick={onRestore}
          >
            <Undo2 />
          </Button>
        </span>
      </Tooltip>
    </motion.li>
  );
}

function Group({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-0.5">
      <h3 className="px-2 pt-2 pb-1 text-[11px] font-medium text-fg-subtle">{label}</h3>
      <ul className="flex flex-col">
        <AnimatePresence initial={false}>{children}</AnimatePresence>
      </ul>
    </section>
  );
}

/** Everything the app currently changes in the game, with a restore per line. */
function useActiveMods() {
  const swaps = useSwaps();
  const palette = usePaletteStatus();
  const decal = useDecalStatus();
  const ball = useBallStatus();
  const map = useMapSession();
  const counts = {
    items: swaps.data?.length ?? 0,
    palette: palette.data?.active ? 1 : 0,
    decal: decal.data?.active ? 1 : 0,
    ball: ball.data?.active ? 1 : 0,
    map: map.data ? 1 : 0,
  };
  const total = counts.items + counts.palette + counts.decal + counts.ball + counts.map;
  const preset = useActivePreset([
    swaps.dataUpdatedAt,
    palette.dataUpdatedAt,
    decal.dataUpdatedAt,
    map.dataUpdatedAt,
  ]);
  return { swaps, palette, decal, ball, map, preset: total > 0 ? preset.data : null, counts, total };
}

function Rail({ onOpen }: { onOpen: () => void }) {
  const { t } = useTranslation();
  const { counts, total } = useActiveMods();
  const marks: { key: keyof typeof counts; icon: LucideIcon }[] = [
    { key: "items", icon: Sparkles },
    { key: "palette", icon: PaletteIcon },
    { key: "decal", icon: Brush },
    { key: "ball", icon: Volleyball },
    { key: "map", icon: MapIcon },
  ];
  return (
    <div className="flex flex-col items-center gap-1 py-3">
      <Tooltip content={t("mods.open")} side="left">
        <Button size="icon-sm" variant="ghost" aria-label={t("mods.open")} onClick={onOpen}>
          <PanelRightOpen />
        </Button>
      </Tooltip>
      {total > 0 ? (
        <button
          type="button"
          onClick={onOpen}
          className="mt-2 flex flex-col items-center gap-2.5 rounded-md px-1.5 py-2 hover:bg-white/[0.04]"
          aria-label={t("mods.count", { count: total })}
        >
          {marks
            .filter((m) => counts[m.key] > 0)
            .map(({ key, icon: Icon }) => (
              <span key={key} className="relative text-[var(--color-accent)]">
                <Icon className="size-4" />
                {counts[key] > 1 ? (
                  <span className="absolute -top-1.5 -right-2 min-w-3.5 rounded-full bg-[var(--color-accent)] px-1 text-center text-[9px] leading-3.5 font-semibold text-white tabular-nums">
                    {counts[key]}
                  </span>
                ) : null}
              </span>
            ))}
        </button>
      ) : null}
    </div>
  );
}

function Content({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation(["common", "items"]);
  const { swaps, palette, decal, ball, map, preset, counts, total } = useActiveMods();
  const running = useGameStatus().data?.running.running ?? false;
  const catalog = useCatalog(counts.items > 0);
  const palettes = usePalettes();
  const restoreSwap = useRestoreSwap();
  const restorePalette = useRestorePalette();
  const removeDecal = useRemoveDecal();
  const removeBall = useRemoveBall();
  const deactivateMap = useDeactivateMap();
  const [confirmAll, setConfirmAll] = useState(false);
  const navigate = useNavigate();
  const editSwap = useUi((s) => s.editSwap);
  const editPreset = useUi((s) => s.editPreset);
  const updatePreset = useUpdatePresetFromCurrent();

  const byId = new Map(catalog.data?.items.map((i) => [i.id, i]));
  const blocked = running ? t("mods.closeGame") : undefined;
  const activePalette = palette.data?.active;
  const paletteColors = palettes.data?.find((p) => p.id === activePalette?.paletteId);
  const activeDecal = decal.data?.active;
  const activeBall = ball.data?.active;
  const session = map.data;
  const maps = useMaps();
  const sessionMap = maps.data?.find((m) => m.id === session?.mapId);
  const sessionPreview = sessionMap
    ? (localPreview(sessionMap.folder, sessionMap.previewFile) ?? sessionMap.previewUrl)
    : null;

  return (
    <div className="flex h-full flex-col" style={{ width: OPEN_WIDTH }}>
      <div className="flex items-center gap-2 border-line border-b px-3 py-2.5">
        <div className="min-w-0 flex-1">
          <p className="text-[13px] font-semibold">{t("mods.title")}</p>
          <p className="text-[11px] text-fg-subtle">
            {total > 0 ? t("mods.count", { count: total }) : t("mods.stock")}
          </p>
        </div>
        <Tooltip content={t("mods.close")} side="left">
          <Button size="icon-sm" variant="ghost" aria-label={t("mods.close")} onClick={onClose}>
            <PanelRightClose />
          </Button>
        </Tooltip>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-1.5 pb-3">
        {total === 0 ? (
          <div className="px-3 py-10 text-center">
            <p className="text-[13px] font-medium">{t("mods.emptyTitle")}</p>
            <p className="mt-1 text-xs text-fg-muted">{t("mods.emptyBody")}</p>
          </div>
        ) : null}

        {preset ? (
          <Group label={t("mods.preset")}>
            <ModRow
              lead={<Layers className="size-4 text-fg-muted" />}
              kicker={t("mods.presetKind")}
              title={preset.name}
              detail={
                preset.modified ? (
                  <span className="text-warning">{t("mods.presetModified")}</span>
                ) : (
                  t("mods.presetApplied", { when: formatRelative(preset.appliedAt) })
                )
              }
              restoreLabel={t("mods.presetRestore")}
              blockedReason={blocked}
              restoring={false}
              onRestore={() => setConfirmAll(true)}
              actions={[
                ...(preset.modified
                  ? [
                      {
                        key: "update",
                        icon: <Save />,
                        label: t("mods.presetUpdate"),
                        loading: updatePreset.isPending,
                        onClick: () => updatePreset.mutate(preset.presetId),
                      },
                    ]
                  : []),
                {
                  key: "edit",
                  icon: <Pencil />,
                  label: t("mods.presetEdit"),
                  onClick: () => {
                    editPreset(preset.presetId);
                    void navigate({ to: "/presets" });
                  },
                },
              ]}
            />
          </Group>
        ) : null}

        {counts.items > 0 ? (
          <Group label={t("mods.items")}>
            {swaps.data?.map((swap) => {
              const Icon = SLOT_ICONS[swap.request.slot];
              return (
                <ModRow
                  key={swap.id}
                  lead={<ItemThumb item={byId.get(swap.request.wantedId)} className="size-9" />}
                  kicker={
                    <>
                      <Icon className="size-3" />
                      {t(`items:slots.${swap.request.slot}`)}
                      <SwapColorBadge paint={swap.request.paint} color={swap.request.color} />
                    </>
                  }
                  title={swap.wantedLabel}
                  detail={t("mods.replaces", { name: swap.ownedLabel })}
                  restoreLabel={t("mods.restore")}
                  blockedReason={blocked}
                  restoring={restoreSwap.isPending && restoreSwap.variables === swap.id}
                  onRestore={() => restoreSwap.mutate(swap.id)}
                  actions={[
                    {
                      key: "edit",
                      icon: <Pencil />,
                      label: t("mods.swapEdit"),
                      onClick: () => {
                        editSwap(swap.id);
                        void navigate({ to: "/items" });
                      },
                    },
                  ]}
                />
              );
            })}
          </Group>
        ) : null}

        {activePalette ? (
          <Group label={t("mods.palette")}>
            <ModRow
              lead={
                <span className="grid size-6 grid-cols-2 overflow-hidden rounded-full border border-white/15">
                  <span style={{ background: rgb(paletteColors?.primaryBlue[0]) }} />
                  <span style={{ background: rgb(paletteColors?.primaryOrange[0]) }} />
                </span>
              }
              kicker={t("mods.paletteKind")}
              title={activePalette.paletteName}
              detail={
                activePalette.stale ? <span className="text-warning">{t("mods.stale")}</span> : undefined
              }
              restoreLabel={t("mods.restore")}
              blockedReason={blocked}
              restoring={restorePalette.isPending}
              onRestore={() =>
                restorePalette.mutate(undefined, { onSuccess: () => notify.success(t("mods.restored")) })
              }
            />
          </Group>
        ) : null}

        {activeDecal ? (
          <Group label={t("mods.decal")}>
            <ModRow
              lead={
                activeDecal.previewPath ? (
                  <img
                    src={convertFileSrc(activeDecal.previewPath)}
                    alt=""
                    draggable={false}
                    className="size-9 rounded-[6px] object-cover"
                  />
                ) : (
                  <Brush className="size-4 text-fg-muted" />
                )
              }
              kicker={
                <>
                  {activeDecal.bodyName}
                  <Badge tone="warning" className="h-4 px-1 text-[10px]">
                    {t("nav.stages.experimental")}
                  </Badge>
                </>
              }
              title={activeDecal.displayName}
              detail={activeDecal.stale ? <span className="text-warning">{t("mods.stale")}</span> : undefined}
              restoreLabel={t("mods.restore")}
              blockedReason={blocked}
              restoring={removeDecal.isPending}
              onRestore={() =>
                removeDecal.mutate(undefined, { onSuccess: () => notify.success(t("mods.restored")) })
              }
            />
          </Group>
        ) : null}

        {activeBall ? (
          <Group label={t("mods.ball")}>
            <ModRow
              lead={
                activeBall.previewPath ? (
                  <img
                    src={convertFileSrc(activeBall.previewPath)}
                    alt=""
                    draggable={false}
                    className="size-9 rounded-full object-cover"
                  />
                ) : (
                  <Volleyball className="size-4 text-fg-muted" />
                )
              }
              kicker={t("mods.ball")}
              title={activeBall.displayName}
              detail={activeBall.stale ? <span className="text-warning">{t("mods.stale")}</span> : undefined}
              restoreLabel={t("mods.restore")}
              blockedReason={blocked}
              restoring={removeBall.isPending}
              onRestore={() =>
                removeBall.mutate(undefined, { onSuccess: () => notify.success(t("mods.restored")) })
              }
            />
          </Group>
        ) : null}

        {session ? (
          <Group label={t("mods.map")}>
            <ModRow
              lead={<MapArt name={session.mapName} src={sessionPreview} className="size-9" />}
              kicker={t("mods.mapKind")}
              title={session.mapName}
              detail={t("mods.replaces", { name: targetLabel(session.target) })}
              restoreLabel={t("mods.restoreMap")}
              restoring={deactivateMap.isPending}
              onRestore={() =>
                deactivateMap.mutate(undefined, {
                  onSuccess: () =>
                    running
                      ? notify.success(t("maps:toast.restoredLive"), t("maps:toast.liveHint"))
                      : notify.success(t("mods.restored")),
                })
              }
            />
          </Group>
        ) : null}
      </div>

      {total > 0 ? (
        <div className="flex flex-col gap-2 border-line border-t p-3">
          {running ? <p className="text-[11px] text-fg-subtle">{t("mods.running")}</p> : null}
          <Button
            size="sm"
            variant="outline"
            className="w-full"
            disabled={running}
            onClick={() => setConfirmAll(true)}
          >
            <RotateCcw />
            {t("mods.restoreAll")}
          </Button>
        </div>
      ) : null}
      <RestoreStockDialog open={confirmAll} onOpenChange={setConfirmAll} />
    </div>
  );
}

/**
 * Right-hand panel listing every change the app made to the game (items,
 * palette, decal, map). Present on every page; folds to a thin rail.
 */
export function ActiveModsPanel() {
  const open = useUi((s) => s.modsPanelOpen);
  const setOpen = useUi((s) => s.setModsPanelOpen);
  return (
    <motion.aside
      animate={{ width: open ? OPEN_WIDTH : RAIL_WIDTH }}
      initial={false}
      transition={{ type: "spring", stiffness: 500, damping: 45 }}
      className="relative z-20 flex shrink-0 flex-col overflow-hidden border-line border-l bg-[rgb(10_11_15/0.55)] backdrop-blur-xl"
    >
      {open ? <Content onClose={() => setOpen(false)} /> : <Rail onOpen={() => setOpen(true)} />}
    </motion.aside>
  );
}
