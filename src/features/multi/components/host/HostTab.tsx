import { useNavigate } from "@tanstack/react-router";
import { ImageOff, Radio, SlidersHorizontal } from "lucide-react";
import { type ReactNode, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { GlassPanel } from "@/components/ui/glass";
import { Segmented } from "@/components/ui/segmented";
import { Select } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { DEFAULT_TARGET, LABS_TARGETS, targetLabel } from "@/features/maps/api";
import { localPreview, MapArt } from "@/features/maps/components/MapArt";
import { useMapSession, useMaps } from "@/features/maps/queries";
import type { MultiStatus } from "../../api";
import { useHost, useRpOptions } from "../../queries";
import { isReady } from "../SetupPanel";
import { Status } from "../Status";
import { MapPickerDialog } from "./MapPickerDialog";
import { MutatorsDialog } from "./MutatorsDialog";
import { PresetBar } from "./PresetBar";
import { useHostForm } from "./useHostForm";

/** Select needs non-empty values for "no choice". */
const KEEP = "__keep";
const PLAYER_OPTIONS = ["auto", "2", "4", "6", "8"] as const;
const ARENA_OPTIONS = LABS_TARGETS.map((t): { value: string; label: string } => ({
  value: t,
  label: targetLabel(t),
}));

function Row({ label, hint, children }: { label: string; hint?: ReactNode; children: ReactNode }) {
  return (
    <div className="grid items-center gap-x-4 gap-y-1 sm:grid-cols-[10rem_minmax(0,1fr)]">
      <div>
        <p className="text-[13px] text-fg">{label}</p>
        {hint ? <p className="text-xs text-fg-subtle">{hint}</p> : null}
      </div>
      <div className="min-w-0">{children}</div>
    </div>
  );
}

export function HostTab({
  status,
  busy,
  onShowInfos,
}: {
  status: MultiStatus | undefined;
  busy: boolean;
  onShowInfos: () => void;
}) {
  const { t } = useTranslation("multi");
  const navigate = useNavigate();
  const maps = useMaps();
  const { data: mapSession } = useMapSession();
  const { data: catalog } = useRpOptions();
  const host = useHost();
  const { form, patch, patchOptions, applyPreset } = useHostForm();
  const [picking, setPicking] = useState(false);
  const [editingMutators, setEditingMutators] = useState(false);

  const list = maps.data ?? [];
  const mapIds = useMemo(() => new Set(list.map((m) => m.id)), [list]);
  const selected = list.find((m) => m.id === form.mapId) ?? null;
  const target = form.target ?? selected?.preferredTarget ?? DEFAULT_TARGET;
  // The form's variant when the map still has it, else the map's default.
  const variant =
    form.variant !== null &&
    selected &&
    (form.variant === 0 || selected.repairs.some((r) => r.version === form.variant))
      ? form.variant
      : (selected?.preferredVariant ?? 0);
  const { options } = form;
  const alreadyThere = Boolean(
    selected &&
      mapSession &&
      mapSession.mapId === selected.id &&
      mapSession.target === target &&
      mapSession.variant === variant,
  );
  const rpPreset = options.preset && catalog?.presets.includes(options.preset) ? options.preset : null;
  const changed = Object.entries(options.mutators);
  const labelOf = (cat: string, value: string) => {
    const m = catalog?.mutators.find((x) => x.category === cat);
    return `${m?.label ?? cat}: ${m?.values.find((v) => v.internal === value)?.display ?? value}`;
  };
  const blocked = !status ? t("play.checking") : !isReady(status) ? t("play.notReady") : null;

  // The form keeps the id of the preset it was loaded from, even after
  // edits, so "update preset" stays one click.
  return (
    <div className="grid gap-4 @4xl:grid-cols-[minmax(0,1fr)_18rem]">
      <GlassPanel className="flex flex-col gap-5 p-4">
        <PresetBar
          form={form}
          target={target}
          mapIds={mapIds}
          onApply={applyPreset}
          onSaved={(p) => patch({ presetId: p.id })}
        />

        <div className="flex flex-col gap-3 border-line border-t pt-4">
          <h3 className="text-xs font-semibold text-fg-muted">{t("host.sections.map")}</h3>
          {list.length ? (
            <button
              type="button"
              onClick={() => setPicking(true)}
              className="group flex items-center gap-3 rounded-[8px] border border-line p-2 text-left transition-colors hover:border-line-strong"
            >
              {selected ? (
                <MapArt
                  name={selected.name}
                  src={localPreview(selected.folder, selected.previewFile) ?? selected.previewUrl}
                  className="aspect-[16/9] w-36 shrink-0 overflow-hidden rounded-[6px]"
                />
              ) : (
                <span className="grid aspect-[16/9] w-36 shrink-0 place-items-center rounded-[6px] bg-white/[0.04] text-fg-subtle">
                  <ImageOff className="size-5" />
                </span>
              )}
              <span className="min-w-0 flex-1">
                <span className="block truncate text-[13px] font-medium text-fg">
                  {selected?.name ?? t("host.pickMap")}
                </span>
                {selected?.author ? (
                  <span className="block truncate text-xs text-fg-subtle">{selected.author}</span>
                ) : null}
              </span>
              <span className="shrink-0 text-xs text-fg-subtle group-hover:text-fg">
                {t("host.changeMap")}
              </span>
            </button>
          ) : (
            <div className="flex items-center justify-between gap-3 rounded-[8px] border border-line p-3">
              <p className="text-xs text-fg-muted">{t("host.noMaps")}</p>
              <Button size="sm" variant="secondary" onClick={() => void navigate({ to: "/maps" })}>
                {t("join.openMaps")}
              </Button>
            </div>
          )}
          <Row label={t("host.arena")} hint={t("host.arenaHint")}>
            <Select value={target} onValueChange={(v) => patch({ target: v })} options={ARENA_OPTIONS} />
          </Row>
          {selected?.repairs.length ? (
            <Row label={t("host.variant")} hint={t("host.variantHint")}>
              <Select
                value={String(variant)}
                onValueChange={(v) => patch({ variant: Number(v) })}
                options={[
                  { value: "0", label: t("host.variantOriginal") },
                  ...selected.repairs.map((r) => ({
                    value: String(r.version),
                    label: t("host.variantRepaired", { version: r.version }),
                  })),
                ]}
              />
            </Row>
          ) : null}
          {selected ? (
            <Status ok={alreadyThere}>
              {alreadyThere
                ? t("host.alreadyThere", { arena: targetLabel(target) })
                : t("host.willInstall", { arena: targetLabel(target) })}
            </Status>
          ) : null}
          {selected && variant > 0 ? (
            <p className="text-xs text-fg-subtle">{t("host.repairedNote", { version: variant })}</p>
          ) : null}
        </div>

        <div className="flex flex-col gap-3 border-line border-t pt-4">
          <h3 className="text-xs font-semibold text-fg-muted">{t("host.sections.match")}</h3>
          <label className="flex items-start justify-between gap-4">
            <span className="min-w-0">
              <span className="block text-[13px] text-fg">{t("options.manual")}</span>
              <span className="block text-xs text-fg-subtle">{t("options.manualHint")}</span>
            </span>
            <Switch checked={options.manual} onCheckedChange={(manual) => patchOptions({ manual })} />
          </label>
          {options.manual || !catalog ? null : (
            <>
              <Row label={t("options.mode")}>
                <Select
                  value={options.gameMode ?? KEEP}
                  onValueChange={(v) => patchOptions({ gameMode: v === KEEP ? null : v })}
                  options={[
                    { value: KEEP, label: t("options.keepMode") },
                    ...catalog.gameModes.map((m) => ({ value: m.internal, label: m.display })),
                  ]}
                />
              </Row>
              <Row label={t("options.players")}>
                <Segmented
                  size="sm"
                  value={options.players ? String(options.players) : "auto"}
                  onValueChange={(v) => patchOptions({ players: v === "auto" ? null : Number(v) })}
                  options={PLAYER_OPTIONS.map((p) => ({
                    value: p,
                    label: p === "auto" ? t("options.auto") : p,
                  }))}
                />
              </Row>
              <Row label={t("options.preset")} hint={t("options.presetHint")}>
                <Select
                  value={rpPreset ?? KEEP}
                  onValueChange={(v) => patchOptions({ preset: v === KEEP ? null : v })}
                  options={[
                    { value: KEEP, label: t("options.noPreset") },
                    ...catalog.presets.map((p) => ({ value: p, label: p })),
                  ]}
                />
              </Row>
              <Row label={t("options.mutators")}>
                <div className="flex flex-wrap items-center gap-1.5">
                  {changed.slice(0, 6).map(([cat, value]) => (
                    <Badge key={cat} tone="accent">
                      {labelOf(cat, value)}
                    </Badge>
                  ))}
                  {changed.length > 6 ? <Badge>+{changed.length - 6}</Badge> : null}
                  <Button size="sm" variant="secondary" onClick={() => setEditingMutators(true)}>
                    <SlidersHorizontal />
                    {changed.length ? t("host.mutators.edit") : t("host.mutators.open")}
                  </Button>
                </div>
              </Row>
              <p className="text-xs text-fg-subtle">{t("options.hint")}</p>
            </>
          )}
        </div>
      </GlassPanel>

      <GlassPanel
        className="flex flex-col gap-3 self-start p-4"
        style={{ ["--color-accent" as string]: "var(--color-team-orange)" }}
      >
        <h3 className="flex items-center gap-2 text-sm font-semibold">
          <Radio className="size-4" style={{ color: "var(--color-team-orange)" }} />
          {t("host.title")}
        </h3>
        <p className="text-xs text-fg-muted">{t("host.description")}</p>
        <p className="text-xs text-fg-subtle">{t("host.arenaShared")}</p>
        <Button
          variant="primary"
          className="w-full"
          disabled={!selected || Boolean(blocked) || busy}
          loading={host.isPending}
          onClick={() =>
            selected &&
            host.mutate({ mapId: selected.id, target, variant, options: { ...options, preset: rpPreset } })
          }
        >
          <Radio />
          {t("host.cta")}
        </Button>
        {blocked ? (
          <button
            type="button"
            onClick={onShowInfos}
            className="text-center text-xs text-warning hover:underline"
          >
            {blocked}
          </button>
        ) : null}
      </GlassPanel>

      <MapPickerDialog
        open={picking}
        onOpenChange={setPicking}
        maps={list}
        selectedId={selected?.id ?? null}
        onPick={(m) => patch({ mapId: m.id, target: null, variant: null })}
      />
      {catalog ? (
        <MutatorsDialog
          open={editingMutators}
          onOpenChange={setEditingMutators}
          catalog={catalog}
          value={options.mutators}
          onChange={(mutators) => patchOptions({ mutators })}
        />
      ) : null}
    </div>
  );
}
