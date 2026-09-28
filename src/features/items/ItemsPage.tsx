import { FileKey2, Lock, PackageSearch, RefreshCw, Sparkles } from "lucide-react";
import { useCallback, useDeferredValue, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useKeysPicker } from "@/components/shell/WelcomeDialog";
import { Button } from "@/components/ui/button";
import { EmptyState, ErrorState, Skeleton } from "@/components/ui/feedback";
import { SearchInput } from "@/components/ui/input";
import { Page, PageHeader } from "@/components/ui/layout";
import { Segmented } from "@/components/ui/segmented";
import { Select } from "@/components/ui/select";
import { notify } from "@/components/ui/toast";
import { Tooltip } from "@/components/ui/tooltip";
import { formatNumber } from "@/lib/format";
import { useUi } from "@/stores/ui";
import type { ActiveSwap, CatalogItem, Slot } from "./api";
import { HistoryDialog } from "./components/HistoryDialog";
import { IntegrityBanner } from "./components/IntegrityBanner";
import { ItemGrid } from "./components/ItemGrid";
import { SlotRail } from "./components/SlotRail";
import { type ComposerMode, type PickStep, SwapComposer } from "./components/SwapComposer";
import { itemLabel, searchKey } from "./constants";
import { useApplySwap, useCatalog, useRefreshCatalog, useSwaps } from "./queries";

type BodyFilter = "auto" | "all" | "universal" | `${number}`;

export function ItemsPage() {
  const { t } = useTranslation("items");
  const catalog = useCatalog();
  const swaps = useSwaps();
  const refresh = useRefreshCatalog();
  const apply = useApplySwap();
  const keysPicker = useKeysPicker();

  const [slot, setSlot] = useState<Slot>("decal");
  const [step, setStep] = useState<PickStep>("owned");
  const [ownedId, setOwnedId] = useState<number | null>(null);
  const [wantedId, setWantedId] = useState<number | null>(null);
  const [paint, setPaint] = useState(0);
  const [tint, setTint] = useState<number | null>(null);
  const [search, setSearch] = useState("");
  const [bodyFilter, setBodyFilter] = useState<BodyFilter>("auto");
  const deferredSearch = useDeferredValue(search);

  const items = catalog.data?.items;
  const byId = useMemo(() => new Map(items?.map((i) => [i.id, i])), [items]);
  const owned = ownedId !== null ? byId.get(ownedId) : undefined;
  const wanted = wantedId !== null ? byId.get(wantedId) : undefined;

  const counts = useMemo(
    () =>
      Object.fromEntries((catalog.data?.counts ?? []).map((c) => [c.slot, c.count])) as Partial<
        Record<Slot, number>
      >,
    [catalog.data],
  );
  const activeSlots = useMemo(() => new Set(swaps.data?.map((s) => s.request.slot)), [swaps.data]);
  const swappedIds = useMemo(() => new Set(swaps.data?.map((s) => s.request.ownedId)), [swaps.data]);

  /** The body the player will *see*: a swapped body wins over the owned one. */
  const displayedBodyId = useMemo(() => {
    const bodySwap = swaps.data?.find((s) => s.request.slot === "body");
    if (bodySwap) return bodySwap.request.wantedId;
    return owned?.slot === "decal" ? owned.bodyId : null;
  }, [swaps.data, owned]);

  const bodies = useMemo(
    () =>
      (items ?? []).filter((i) => i.slot === "body").sort((a, b) => itemLabel(a).localeCompare(itemLabel(b))),
    [items],
  );

  const effectiveBody: number | "all" | "universal" =
    bodyFilter === "auto"
      ? step === "wanted" && displayedBodyId != null
        ? displayedBodyId
        : "all"
      : bodyFilter === "all" || bodyFilter === "universal"
        ? bodyFilter
        : Number(bodyFilter);

  const visible = useMemo(() => {
    const q = searchKey(deferredSearch.trim());
    return (items ?? []).filter((i) => {
      if (i.slot !== slot) return false;
      if (slot === "decal" && effectiveBody !== "all") {
        if (
          effectiveBody === "universal" ? i.bodyId !== null : i.bodyId !== null && i.bodyId !== effectiveBody
        ) {
          return false;
        }
      }
      if (!q) return true;
      // Labels in both languages + internal id + file name (items without a
      // translated name are still findable, e.g. "dev" → boost_alphadevreward).
      return searchKey(`${i.labelFr} ${i.labelEn} ${i.asset} ${i.package}`).includes(q);
    });
  }, [items, slot, deferredSearch, effectiveBody]);

  const disabledIds = useMemo(() => {
    if (step !== "wanted" || !owned) return new Set<number>();
    const pkg = owned.package.toLowerCase();
    // Same package as the owned item (locked items stay clickable: picking one explains why it can't be used).
    // The owned item itself stays pickable: recolouring it in place.
    return new Set(
      visible.filter((i) => i.package.toLowerCase() === pkg && i.id !== owned.id).map((i) => i.id),
    );
  }, [step, owned, visible]);

  /** Loads an active swap into the composer (owned, wanted and paint). */
  const loadSwap = useCallback((swap: ActiveSwap) => {
    setSlot(swap.request.slot);
    setOwnedId(swap.request.ownedId);
    setWantedId(swap.request.wantedId);
    setPaint(swap.request.paint ?? 0);
    setTint(swap.request.tint ?? null);
    setStep("wanted");
    setSearch("");
    setBodyFilter("auto");
  }, []);

  const changeSlot = (next: Slot) => {
    // Reopen the active swap of this slot, if any, ready to be edited.
    const previous = swaps.data?.find((s) => s.request.slot === next);
    if (previous) return loadSwap(previous);
    setSlot(next);
    setSearch("");
    setWantedId(null);
    setPaint(0);
    setTint(null);
    setBodyFilter("auto");
    setOwnedId(null);
    setStep("owned");
  };

  // First visit: open the active swap of the default slot, if any.
  const [primed, setPrimed] = useState(false);
  useEffect(() => {
    if (primed || !swaps.data) return;
    setPrimed(true);
    const current = swaps.data.find((s) => s.request.slot === slot);
    if (current && ownedId === null) loadSwap(current);
  }, [primed, swaps.data, slot, ownedId, loadSwap]);

  // "Edit" from the active mods panel.
  const swapToEdit = useUi((s) => s.swapToEdit);
  const editSwap = useUi((s) => s.editSwap);
  useEffect(() => {
    if (!swapToEdit || !swaps.data) return;
    const swap = swaps.data.find((s) => s.id === swapToEdit);
    if (swap) loadSwap(swap);
    editSwap(null);
  }, [swapToEdit, swaps.data, loadSwap, editSwap]);

  const activeForOwned = swaps.data?.find((s) => s.request.slot === slot && s.request.ownedId === ownedId);
  const mode: ComposerMode = !activeForOwned
    ? "new"
    : activeForOwned.request.wantedId === wantedId &&
        (activeForOwned.request.paint ?? 0) === paint &&
        (activeForOwned.request.tint ?? null) === tint
      ? "unchanged"
      : "update";

  const pick = (item: CatalogItem) => {
    if (item.locked) {
      notify.info(t("locked.pickTitle"), t("locked.pickBody", { name: itemLabel(item) }));
      return;
    }
    if (step === "owned") {
      const existing = swaps.data?.find((s) => s.request.slot === slot && s.request.ownedId === item.id);
      if (existing) return loadSwap(existing);
      setOwnedId(item.id);
      if (wantedId !== null && byId.get(wantedId)?.package === item.package) setWantedId(null);
      setStep("wanted");
      setSearch("");
    } else {
      setWantedId(item.id);
    }
  };

  const lockedInSlot = useMemo(
    () => (items ?? []).filter((i) => i.slot === slot && i.locked).length,
    [items, slot],
  );

  const onApply = () => {
    if (!owned || !wanted) return;
    // The pick stays: the composer then shows it as the active swap.
    apply.mutate({ slot, ownedId: owned.id, wantedId: wanted.id, paint: paint > 0 ? paint : null, tint });
  };

  const bodyOptions = [
    { value: "auto" as BodyFilter, label: t("filters.bodyAuto") },
    { value: "all" as BodyFilter, label: t("filters.bodyAll") },
    { value: "universal" as BodyFilter, label: t("filters.bodyUniversal") },
    ...bodies.map((b) => ({ value: String(b.id) as BodyFilter, label: itemLabel(b) })),
  ];

  return (
    <Page className="max-w-none">
      <PageHeader
        icon={Sparkles}
        eyebrow={t("eyebrow")}
        title={t("title")}
        subtitle={t("subtitle")}
        actions={
          <>
            <HistoryDialog />
            <Tooltip content={t("refreshHint")}>
              <Button variant="secondary" loading={refresh.isPending} onClick={() => refresh.mutate()}>
                <RefreshCw />
                {t("refresh")}
              </Button>
            </Tooltip>
          </>
        }
      />

      <IntegrityBanner />

      {catalog.data && !catalog.data.fromGame ? (
        <div className="flex items-center gap-3 rounded-lg border border-warning/25 bg-warning/[0.06] px-3.5 py-2.5">
          <FileKey2 className="size-4 shrink-0 text-warning" />
          <div className="min-w-0 flex-1">
            <p className="text-[13px] font-medium">{t("noKeys.title")}</p>
            <p className="text-xs text-fg-muted">{t("noKeys.description")}</p>
          </div>
          <Button
            variant="primary"
            size="sm"
            loading={keysPicker.busy}
            onClick={() => void keysPicker.pick()}
          >
            <FileKey2 />
            {t("noKeys.import")}
          </Button>
        </div>
      ) : null}

      {catalog.error ? (
        <ErrorState error={catalog.error} onRetry={() => void catalog.refetch()} />
      ) : (
        <div className="grid h-[calc(100vh-13rem)] min-h-[540px] grid-cols-[208px_minmax(0,1fr)] gap-4">
          <div className="flex min-h-0 flex-col gap-3">
            <SlotRail value={slot} onChange={changeSlot} counts={counts} activeSlots={activeSlots} />
            {catalog.data ? (
              <p className="px-2 text-[11px] leading-relaxed text-fg-subtle">
                {t("catalogStats", {
                  items: formatNumber(catalog.data.items.length),
                  missing: formatNumber(catalog.data.unresolved),
                })}
                {catalog.data.fromGame ? (
                  <>
                    <br />
                    {t("catalogFromGame")}
                  </>
                ) : null}
              </p>
            ) : null}
          </div>

          <section className="flex min-h-0 flex-col gap-3">
            <SwapComposer
              owned={owned}
              wanted={wanted}
              step={step}
              onStepChange={setStep}
              paint={paint}
              onPaintChange={setPaint}
              tint={tint}
              onTintChange={setTint}
              onApply={onApply}
              applying={apply.isPending}
              mode={mode}
            />

            <div className="flex flex-wrap items-center gap-2">
              <Segmented
                aria-label={t("steps.label")}
                value={step}
                onValueChange={setStep}
                options={[
                  { value: "owned", label: t("steps.owned") },
                  { value: "wanted", label: t("steps.wanted") },
                ]}
              />
              <SearchInput
                value={search}
                onValueChange={setSearch}
                placeholder={t("searchPlaceholder", { slot: t(`slots.${slot}`).toLowerCase() })}
                className="min-w-56 flex-1"
              />
              {slot === "decal" ? (
                <Select
                  value={bodyFilter}
                  onValueChange={setBodyFilter}
                  options={bodyOptions}
                  className="w-56"
                />
              ) : null}
              <span className="text-xs tabular-nums text-fg-subtle">
                {t("resultCount", { count: visible.length })}
              </span>
            </div>

            {lockedInSlot > 0 ? (
              <div className="flex items-start gap-2 rounded-lg border border-warning/25 bg-warning/[0.06] px-3 py-2 text-xs">
                <Lock className="mt-0.5 size-3.5 shrink-0 text-warning" />
                <p className="text-fg-muted">
                  <span className="font-medium text-warning">
                    {t("locked.bannerTitle", { count: lockedInSlot })}
                  </span>{" "}
                  {t("locked.bannerBody")}
                </p>
              </div>
            ) : null}

            <div className="min-h-0 flex-1">
              {catalog.isLoading ? (
                <div className="grid grid-cols-5 gap-3">
                  {Array.from({ length: 15 }, (_, i) => (
                    // biome-ignore lint/suspicious/noArrayIndexKey: static placeholders
                    <Skeleton key={i} className="h-44" />
                  ))}
                </div>
              ) : visible.length === 0 ? (
                <EmptyState
                  icon={PackageSearch}
                  title={t("empty.title")}
                  description={t("empty.description")}
                />
              ) : (
                <ItemGrid
                  items={visible}
                  selectedId={step === "owned" ? ownedId : wantedId}
                  disabledIds={disabledIds}
                  swappedIds={swappedIds}
                  dimLocked={step === "wanted"}
                  onSelect={pick}
                />
              )}
            </div>
          </section>
        </div>
      )}
    </Page>
  );
}
