import { Link2, Link2Off, Palette as PaletteIcon, RotateCcw, Save, Sparkles } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { ErrorState, Skeleton } from "@/components/ui/feedback";
import { GlassCard, GlassPanel } from "@/components/ui/glass";
import { Input } from "@/components/ui/input";
import { Page, PageHeader, Section } from "@/components/ui/layout";
import { Segmented } from "@/components/ui/segmented";
import { Switch } from "@/components/ui/switch";
import { notify } from "@/components/ui/toast";
import { Tooltip } from "@/components/ui/tooltip";
import { useGameStatus } from "@/lib/game";
import { useFx } from "@/stores/fx";
import { type Palette, type PaletteDraft, type Rgb, STOCK_ID } from "./api";
import { ACCENT_COLUMNS, auroraFrom, normalize, PICKER_ROWS, PRIMARY_COLUMNS } from "./colors";
import { ColorEditor } from "./components/ColorEditor";
import { Generators } from "./components/Generators";
import { PaletteLibrary } from "./components/PaletteLibrary";
import { PaletteStatusBanner } from "./components/PaletteStatusBanner";
import { SwatchGrid } from "./components/SwatchGrid";
import {
  useApplyPalette,
  useDeletePalette,
  usePaletteStatus,
  usePalettes,
  useRestorePalette,
  useSavePalette,
  useStockPalette,
} from "./queries";

type Team = "blue" | "orange" | "accent";
const FIELD: Record<Team, keyof Pick<PaletteDraft, "primaryBlue" | "primaryOrange" | "accent">> = {
  blue: "primaryBlue",
  orange: "primaryOrange",
  accent: "accent",
};

const sameColors = (a: readonly Rgb[], b: readonly Rgb[]) =>
  a.length === b.length && a.every((c, i) => c.r === b[i]?.r && c.g === b[i]?.g && c.b === b[i]?.b);

export function PalettePage() {
  const { t } = useTranslation("palette");
  const status = usePaletteStatus();
  const stock = useStockPalette(status.data?.engine === "ready");
  const palettes = usePalettes();
  const game = useGameStatus();
  const save = useSavePalette();
  const apply = useApplyPalette();
  const restore = useRestorePalette();
  const remove = useDeletePalette();
  const setAccents = useFx((s) => s.setAccents);
  const resetAccents = useFx((s) => s.resetAccents);

  const [draft, setDraft] = useState<PaletteDraft | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [team, setTeam] = useState<Team>("blue");
  const [linked, setLinked] = useState(true);
  const [slot, setSlot] = useState<number | null>(null);
  const [pending, setPending] = useState<Palette | null>(null);
  const [toDelete, setToDelete] = useState<Palette | null>(null);

  const gameRunning = Boolean(game.data?.running.running);
  const activeId = status.data?.active?.paletteId ?? null;
  const columns = team === "accent" ? ACCENT_COLUMNS : PRIMARY_COLUMNS;
  const slotCount = columns * PICKER_ROWS;

  /** Loads a palette into the editor (stock → a new, unsaved copy). */
  const load = (p: Palette, asCopy = false) => {
    const base = stock.data;
    if (!base) return;
    const isStock = p.id === STOCK_ID;
    const blue = normalize(p.primaryBlue, base.primaryBlue, PRIMARY_COLUMNS * PICKER_ROWS);
    const orange = normalize(
      p.primaryOrange.length ? p.primaryOrange : p.primaryBlue,
      base.primaryOrange,
      PRIMARY_COLUMNS * PICKER_ROWS,
    );
    setDraft({
      id: isStock || asCopy ? "" : p.id,
      name: isStock ? t("editor.newName") : asCopy ? t("editor.copyName", { name: p.name }) : p.name,
      primaryBlue: blue,
      primaryOrange: orange,
      accent: normalize(p.accent, base.accent, ACCENT_COLUMNS * PICKER_ROWS),
    });
    setSelectedId(asCopy ? null : p.id);
    setLinked(sameColors(blue, orange));
    setDirty(asCopy);
    setSlot(null);
  };

  // First load: open the palette currently in game, or the stock one.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `load` is stable enough; runs until a draft exists
  useEffect(() => {
    if (draft || !stock.data || !palettes.data) return;
    const active = palettes.data.find((p) => p.id === activeId);
    load(active ?? stock.data);
  }, [stock.data, palettes.data, activeId, draft]);

  // The aurora takes the palette's most vivid colours while editing.
  useEffect(() => {
    if (draft) setAccents(auroraFrom(draft.primaryBlue, draft.primaryOrange, draft.accent));
  }, [draft, setAccents]);
  useEffect(() => () => resetAccents(), [resetAccents]);

  const current = useMemo(() => (draft ? draft[FIELD[team]] : []), [draft, team]);
  const stockColors = stock.data ? stock.data[FIELD[team]] : [];

  const update = (next: Rgb[]) => {
    setDraft((d) => {
      if (!d) return d;
      const out = { ...d, [FIELD[team]]: next };
      if (linked && team !== "accent") {
        out.primaryBlue = next;
        out.primaryOrange = next;
      }
      return out;
    });
    setDirty(true);
  };

  const setSlotColor = (index: number, color: Rgb) =>
    update(current.map((c, i) => (i === index ? color : c)));

  const fill = (mode: "row" | "column") => {
    if (slot === null) return;
    const color = current[slot];
    if (!color) return;
    const row = Math.floor(slot / columns);
    const col = slot % columns;
    update(
      current.map((c, i) =>
        (mode === "row" ? Math.floor(i / columns) === row : i % columns === col) ? color : c,
      ),
    );
  };

  const persist = async (): Promise<Palette | null> => {
    if (!draft) return null;
    const saved = await save.mutateAsync(draft);
    setDraft({ ...draft, id: saved.id, name: saved.name });
    setSelectedId(saved.id);
    setDirty(false);
    return saved;
  };

  const onSave = async () => {
    const saved = await persist().catch(() => null);
    if (saved) notify.success(t("toast.saved", { name: saved.name }));
  };

  const onApply = async () => {
    if (!draft) return;
    try {
      const target = dirty || !draft.id ? await persist() : { id: draft.id, name: draft.name };
      if (!target) return;
      await apply.mutateAsync(target.id);
      notify.success(t("toast.applied", { name: target.name }), t("toast.restartHint"));
    } catch {
      // errors are surfaced by the mutations
    }
  };

  const select = (p: Palette) => {
    if (dirty) setPending(p);
    else load(p);
  };

  const engineReady = status.data?.engine === "ready";
  const busy = save.isPending || apply.isPending || restore.isPending;

  const teamOptions = [
    {
      value: "blue" as const,
      label: t("editor.teams.blue"),
      icon: <span className="size-2.5 rounded-full bg-team-blue" />,
    },
    {
      value: "orange" as const,
      label: t("editor.teams.orange"),
      icon: <span className="size-2.5 rounded-full bg-team-orange" />,
    },
    {
      value: "accent" as const,
      label: t("editor.teams.accent"),
      icon: (
        <span className="size-2.5 rounded-full bg-[conic-gradient(#ff5d73,#ffc53d,#3ddc97,#5cc8ff,#7c6cff,#ff5d73)]" />
      ),
    },
  ];

  return (
    <Page>
      <PageHeader
        icon={PaletteIcon}
        eyebrow={t("eyebrow")}
        title={t("title")}
        subtitle={t("subtitle")}
        actions={
          status.data?.active ? (
            <Button
              variant="ghost"
              onClick={() => restore.mutate()}
              loading={restore.isPending}
              disabled={gameRunning}
            >
              <RotateCcw />
              {t("actions.restore")}
            </Button>
          ) : null
        }
      />

      <PaletteStatusBanner status={status.data} gameRunning={gameRunning} />
      {status.error ? <ErrorState error={status.error} onRetry={() => void status.refetch()} /> : null}
      {stock.error ? <ErrorState error={stock.error} onRetry={() => void stock.refetch()} /> : null}

      <Section title={t("sections.library")} description={t("sections.libraryHint")}>
        <PaletteLibrary
          palettes={palettes.data}
          stock={stock.data}
          selectedId={selectedId}
          activeId={activeId}
          onSelect={select}
          onNew={() => stock.data && load(stock.data)}
          onDuplicate={(p) => load(p, true)}
          onDelete={setToDelete}
        />
      </Section>

      <Section
        title={
          draft
            ? t("sections.editor", { name: draft.name || t("editor.newName") })
            : t("sections.editorEmpty")
        }
        description={t("sections.editorHint")}
        actions={
          <div className="flex items-center gap-2">
            {dirty ? (
              <Badge tone="warning" dot>
                {t("editor.unsaved")}
              </Badge>
            ) : draft?.id && draft.id === activeId ? (
              <Badge tone="success" dot>
                {t("library.inGame")}
              </Badge>
            ) : null}
            <Button variant="secondary" onClick={onSave} loading={save.isPending} disabled={!draft || !dirty}>
              <Save />
              {t("actions.save")}
            </Button>
            <Tooltip content={gameRunning ? t("status.gameRunning") : t("actions.applyHint")}>
              <span>
                <Button
                  variant="primary"
                  onClick={onApply}
                  loading={apply.isPending}
                  disabled={!draft || !engineReady || gameRunning || busy}
                >
                  <Sparkles />
                  {t("actions.apply")}
                </Button>
              </span>
            </Tooltip>
          </div>
        }
      >
        <div className="grid gap-5 @5xl:grid-cols-[minmax(0,1fr)_300px]">
          <GlassCard interactive={false} className="flex min-w-0 flex-col gap-4 p-4">
            {!draft ? (
              <div className="space-y-4">
                <Skeleton className="h-10 w-72" />
                <Skeleton className="aspect-[10/7] w-full" />
              </div>
            ) : (
              <>
                <div className="flex flex-wrap items-end gap-4">
                  <label className="flex min-w-56 flex-1 flex-col gap-1">
                    <span className="text-xs text-fg-subtle">{t("editor.nameLabel")}</span>
                    <Input
                      value={draft.name}
                      onChange={(e) => {
                        setDraft({ ...draft, name: e.target.value });
                        setDirty(true);
                      }}
                      placeholder={t("editor.namePlaceholder")}
                      className="max-w-sm text-sm font-semibold"
                      maxLength={60}
                    />
                  </label>
                  <div className="flex items-center gap-2.5 pb-1.5">
                    <Tooltip content={t("editor.linkHint")}>
                      <span className="flex items-center gap-2 text-xs text-fg-muted">
                        {linked ? <Link2 className="size-3.5 text-fg" /> : <Link2Off className="size-3.5" />}
                        {t("editor.link")}
                      </span>
                    </Tooltip>
                    <Switch
                      checked={linked}
                      onCheckedChange={(v) => {
                        setLinked(v);
                        if (v && draft) {
                          const source = team === "orange" ? draft.primaryOrange : draft.primaryBlue;
                          setDraft({ ...draft, primaryBlue: source, primaryOrange: source });
                          setDirty(true);
                        }
                      }}
                      aria-label={t("editor.link")}
                    />
                  </div>
                </div>

                <div className="flex flex-col gap-2">
                  <span className="text-xs text-fg-subtle">{t("editor.teamLabel")}</span>
                  <Segmented
                    value={team}
                    onValueChange={(v) => {
                      setTeam(v);
                      setSlot(null);
                    }}
                    options={teamOptions}
                    aria-label={t("editor.teamLabel")}
                  />
                </div>

                <div className="rounded-[8px] border border-line bg-black/20 p-2.5">
                  <SwatchGrid
                    key={team}
                    colors={current}
                    columns={columns}
                    rows={PICKER_ROWS}
                    selected={slot}
                    onSelect={setSlot}
                    stock={stockColors}
                  />
                </div>
                <p className="text-xs text-fg-subtle">{t("editor.note")}</p>

                <div className="flex flex-col gap-2 border-line border-t pt-4">
                  <div>
                    <p className="text-[13px] font-medium">{t("generators.title")}</p>
                    <p className="text-xs text-fg-subtle">{t("generators.hint")}</p>
                  </div>
                  <Generators
                    columns={columns}
                    rows={PICKER_ROWS}
                    current={current}
                    stock={normalize(stockColors, stockColors, slotCount)}
                    onReplace={(next) => update(next.slice(0, slotCount))}
                  />
                </div>
              </>
            )}
          </GlassCard>

          <GlassPanel className="h-fit @5xl:sticky @5xl:top-4">
            <ColorEditor
              index={slot}
              color={slot !== null ? (current[slot] ?? null) : null}
              stockColor={slot !== null ? (stockColors[slot] ?? null) : null}
              columns={columns}
              onChange={(c) => slot !== null && setSlotColor(slot, c)}
              onFillRow={() => fill("row")}
              onFillColumn={() => fill("column")}
            />
          </GlassPanel>
        </div>
      </Section>

      <Dialog open={pending !== null} onOpenChange={(o) => !o && setPending(null)}>
        <DialogContent
          size="sm"
          title={t("dialogs.discardTitle")}
          description={t("dialogs.discardBody")}
          footer={
            <>
              <Button variant="ghost" onClick={() => setPending(null)}>
                {t("common:actions.cancel")}
              </Button>
              <Button
                variant="danger"
                onClick={() => {
                  if (pending) load(pending);
                  setPending(null);
                }}
              >
                {t("dialogs.discard")}
              </Button>
            </>
          }
        />
      </Dialog>

      <Dialog open={toDelete !== null} onOpenChange={(o) => !o && setToDelete(null)}>
        <DialogContent
          size="sm"
          title={t("dialogs.deleteTitle", { name: toDelete?.name ?? "" })}
          description={toDelete?.id === activeId ? t("dialogs.deleteActiveBody") : t("dialogs.deleteBody")}
          footer={
            <>
              <Button variant="ghost" onClick={() => setToDelete(null)}>
                {t("common:actions.cancel")}
              </Button>
              <Button
                variant="danger"
                loading={remove.isPending}
                onClick={() => {
                  if (!toDelete) return;
                  const id = toDelete.id;
                  remove.mutate(id, {
                    onSuccess: () => {
                      if (selectedId === id && stock.data) load(stock.data);
                      setToDelete(null);
                    },
                  });
                }}
              >
                {t("common:actions.delete")}
              </Button>
            </>
          }
        />
      </Dialog>
    </Page>
  );
}
