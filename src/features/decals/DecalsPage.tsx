import { AlertTriangle, Brush, FolderInput, KeyRound, Paintbrush, RefreshCw, Trash2 } from "lucide-react";
import { motion } from "motion/react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { EmptyState, ErrorState, Skeleton } from "@/components/ui/feedback";
import { GlassCard, GlassPanel } from "@/components/ui/glass";
import { SearchInput } from "@/components/ui/input";
import { Page, PageHeader, Section } from "@/components/ui/layout";
import { Segmented } from "@/components/ui/segmented";
import { notify } from "@/components/ui/toast";
import { BakkesImportDialog } from "@/features/extras/components/BakkesImportDialog";
import { formatRelative } from "@/lib/format";
import { useConfig, useGameStatus } from "@/lib/game";
import type { DecalPack } from "./api";
import { ApplyDialog } from "./components/ApplyDialog";
import { LibraryFolders } from "./components/LibraryFolders";
import { assetUrl, PackCard, PackPreview } from "./components/PackCard";
import {
  useApplyDecal,
  useDecalLibrary,
  useDecalStatus,
  useRemoveDecal,
  useSetDecalFolders,
} from "./queries";

type Filter = "supported" | "all";

export function DecalsPage() {
  const { t } = useTranslation("decals");
  const library = useDecalLibrary();
  const status = useDecalStatus();
  const config = useConfig();
  const game = useGameStatus();
  const apply = useApplyDecal();
  const remove = useRemoveDecal();
  const setFolders = useSetDecalFolders();

  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<Filter>("supported");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const pendingImport = status.data?.alphaConsole?.notImported ?? 0;

  const gameRunning = Boolean(game.data?.running.running);
  const active = status.data?.active ?? null;

  const packs = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (library.data ?? []).filter(
      (p) =>
        (filter === "all" || p.supported) &&
        (!q || `${p.displayName} ${p.packName} ${p.group} ${p.bodyFolder}`.toLowerCase().includes(q)),
    );
  }, [library.data, query, filter]);

  const selected = library.data?.find((p) => p.id === selectedId) ?? null;
  const activePack = library.data?.find((p) => p.id === active?.packId) ?? null;
  const supportedCount = library.data?.filter((p) => p.supported).length ?? 0;

  const blocked = !status.data
    ? null
    : !status.data.gameFound
      ? t("blocked.game")
      : !status.data.keysPresent
        ? t("blocked.keys")
        : gameRunning
          ? t("blocked.running")
          : null;

  const onApply = (pack: DecalPack, target: string) => {
    apply.mutate(
      { packId: pack.id, target },
      {
        onSuccess: (s) => {
          setSelectedId(null);
          notify.success(
            t("toast.applied", { name: pack.displayName }),
            s.active?.target
              ? t("panel.equipHint", { decal: s.active.target.label, body: s.active.target.bodyName })
              : undefined,
          );
        },
      },
    );
  };

  return (
    <Page>
      <PageHeader
        icon={Brush}
        eyebrow={t("eyebrow")}
        title={
          <span className="flex items-center gap-2.5">
            {t("title")}
            <Badge tone="warning">{t("stage.badge")}</Badge>
          </span>
        }
        subtitle={t("subtitle")}
        actions={
          <Button variant="ghost" onClick={() => void library.refetch()} loading={library.isFetching}>
            <RefreshCw />
            {t("actions.rescan")}
          </Button>
        }
      />

      {pendingImport > 0 ? (
        <GlassPanel className="flex flex-wrap items-center gap-3 p-3 text-[13px]">
          <FolderInput className="size-4 text-fg-subtle" />
          <p className="min-w-0 flex-1 text-fg-muted">{t("import.banner", { count: pendingImport })}</p>
          <Button size="sm" variant="primary" onClick={() => setImporting(true)}>
            {t("import.action")}
          </Button>
        </GlassPanel>
      ) : null}

      <LibraryFolders
        roots={status.data?.libraryRoots ?? []}
        configured={config.data?.decalLibraryFolders ?? []}
        busy={setFolders.isPending}
        onChange={(folders) => setFolders.mutate(folders)}
      />

      <div className="flex items-start gap-2.5 rounded-lg border border-warning/25 bg-warning/[0.06] p-3 text-[13px]">
        <AlertTriangle className="mt-0.5 size-4 shrink-0 text-warning" />
        <p className="text-fg-muted">{t("stage.notice")}</p>
      </div>

      {/* How it works — paintable mode only, stated up front. */}
      <GlassPanel className="p-4">
        <div className="flex flex-wrap items-start gap-6">
          <div className="flex items-start gap-2.5">
            <Paintbrush className="mt-0.5 size-4 text-fg-subtle" />
            <div>
              <p className="text-sm font-semibold">{t("how.title")}</p>
              <p className="text-xs text-fg-muted">{t("how.mode")}</p>
            </div>
          </div>
          <ol className="grid flex-1 gap-3 text-[13px] @2xl:grid-cols-3">
            {(["step1", "step2", "step3"] as const).map((step, i) => (
              <li key={step} className="flex gap-2.5">
                <span className="grid size-5 shrink-0 place-items-center rounded-[5px] border border-line bg-white/[0.04] text-[11px] font-medium text-fg-muted tabular-nums">
                  {i + 1}
                </span>
                <span className="text-fg-muted">{t(`how.${step}`)}</span>
              </li>
            ))}
          </ol>
        </div>
        <p className="mt-3 flex items-start gap-2 text-xs text-fg-subtle">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0 text-warning" />
          {t("how.limit")}
        </p>
      </GlassPanel>

      {status.data && !status.data.keysPresent ? (
        <GlassPanel className="flex items-center gap-2.5 border-warning/25 p-3 text-[13px]">
          <KeyRound className="size-4 text-warning" />
          {t("blocked.keys")}
        </GlassPanel>
      ) : null}

      {active ? (
        <GlassCard interactive={false} selected className="flex flex-wrap items-center gap-4 p-4">
          <PackPreview
            src={assetUrl(activePack?.previewPath ?? null)}
            alt={active.displayName}
            className="size-16 rounded-md"
          />
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <p className="truncate font-semibold">{active.displayName}</p>
              <Badge tone={active.stale ? "warning" : "success"} dot pulse={!active.stale}>
                {active.stale ? t("active.stale") : t("active.inGame")}
              </Badge>
            </div>
            <p className="mt-0.5 text-xs text-fg-muted">
              {active.target
                ? t("panel.equipHint", { decal: active.target.label, body: active.target.bodyName })
                : active.bodyName}{" "}
              · {formatRelative(active.appliedAt)}
            </p>
          </div>
          <Button
            variant="danger"
            onClick={() => remove.mutate()}
            loading={remove.isPending}
            disabled={gameRunning}
          >
            <Trash2 />
            {t("actions.remove")}
          </Button>
        </GlassCard>
      ) : null}

      <Section
        title={t("library.title")}
        description={t("library.count", { count: library.data?.length ?? 0, supported: supportedCount })}
        actions={
          <div className="flex items-center gap-2">
            <SearchInput
              value={query}
              onValueChange={setQuery}
              placeholder={t("library.search")}
              className="w-56"
            />
            <Segmented
              size="sm"
              value={filter}
              onValueChange={setFilter}
              options={[
                { value: "supported", label: t("library.filterSupported") },
                { value: "all", label: t("library.filterAll") },
              ]}
            />
          </div>
        }
      >
        {library.error ? <ErrorState error={library.error} onRetry={() => void library.refetch()} /> : null}
        {library.isLoading ? (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(190px,1fr))] gap-4">
            {Array.from({ length: 6 }, (_, i) => (
              // biome-ignore lint/suspicious/noArrayIndexKey: placeholders
              <Skeleton key={i} className="aspect-[4/3.6]" />
            ))}
          </div>
        ) : packs.length === 0 ? (
          <GlassPanel>
            <EmptyState
              icon={Brush}
              title={library.data?.length ? t("library.noMatch") : t("library.empty")}
              description={library.data?.length ? undefined : t("library.emptyHint")}
            />
          </GlassPanel>
        ) : (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(190px,1fr))] gap-4">
            {packs.map((pack, i) => (
              <motion.div
                key={pack.id}
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: Math.min(i * 0.03, 0.4), type: "spring", stiffness: 380, damping: 30 }}
              >
                <PackCard
                  pack={pack}
                  selected={pack.id === selectedId}
                  active={pack.id === active?.packId}
                  onSelect={() => setSelectedId(pack.id)}
                />
              </motion.div>
            ))}
          </div>
        )}
      </Section>

      <BakkesImportDialog open={importing} onOpenChange={setImporting} focus="decals" />

      <ApplyDialog
        pack={selected}
        targets={status.data?.targets ?? []}
        blocked={blocked}
        applying={apply.isPending}
        onApply={onApply}
        onClose={() => setSelectedId(null)}
      />
    </Page>
  );
}
