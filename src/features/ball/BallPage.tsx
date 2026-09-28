import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  AlertTriangle,
  FolderInput,
  FolderOpen,
  RefreshCw,
  Sparkles,
  Trash2,
  Volleyball,
} from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { EmptyState, ErrorState, Skeleton } from "@/components/ui/feedback";
import { GlassCard, GlassPanel } from "@/components/ui/glass";
import { Page, PageHeader, Section } from "@/components/ui/layout";
import { notify } from "@/components/ui/toast";
import { assetUrl, PackPreview } from "@/features/decals/components/PackCard";
import { BakkesImportDialog } from "@/features/extras/components/BakkesImportDialog";
import { formatRelative } from "@/lib/format";
import { useGameStatus } from "@/lib/game";
import type { BallPack } from "./api";
import { useApplyBall, useBallLibrary, useBallStatus, useRemoveBall } from "./queries";

function BallCard({
  pack,
  active,
  blocked,
  applying,
  onApply,
}: {
  pack: BallPack;
  active: boolean;
  blocked: string | null;
  applying: boolean;
  onApply: () => void;
}) {
  const { t } = useTranslation("ball");
  return (
    <GlassCard interactive={false} selected={active} className="overflow-hidden">
      <div className="relative">
        <PackPreview src={assetUrl(pack.previewPath)} alt={pack.displayName} className="aspect-square" />
        <div className="absolute top-2 left-2 flex gap-1.5">
          {active ? (
            <Badge tone="success" dot>
              {t("inGame")}
            </Badge>
          ) : null}
          {!pack.supported ? <Badge tone="neutral">{t("noImage")}</Badge> : null}
        </div>
      </div>
      <div className="flex items-center gap-2 p-3">
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-semibold">{pack.displayName}</p>
          <p className="truncate text-[11px] text-fg-subtle">{pack.group || pack.packName}</p>
        </div>
        <Button
          size="sm"
          variant={active ? "secondary" : "primary"}
          disabled={!pack.supported || Boolean(blocked)}
          title={blocked ?? undefined}
          loading={applying}
          onClick={onApply}
        >
          <Sparkles />
          {t("apply")}
        </Button>
      </div>
    </GlassCard>
  );
}

export function BallPage() {
  const { t } = useTranslation("ball");
  const library = useBallLibrary();
  const status = useBallStatus();
  const game = useGameStatus();
  const apply = useApplyBall();
  const remove = useRemoveBall();
  const [importing, setImporting] = useState(false);
  const [applyingId, setApplyingId] = useState<string | null>(null);

  const gameRunning = Boolean(game.data?.running.running);
  const active = status.data?.active ?? null;
  const pending = status.data?.alphaConsole?.notImported ?? 0;
  const firstPack = library.data?.[0];
  const blocked = !status.data
    ? null
    : !status.data.gameFound
      ? t("blocked.game")
      : !status.data.keysPresent
        ? t("blocked.keys")
        : gameRunning
          ? t("blocked.running")
          : null;

  const onApply = (pack: BallPack) => {
    setApplyingId(pack.id);
    apply.mutate(pack.id, {
      onSuccess: () => notify.success(t("applied", { name: pack.displayName }), t("appliedHint")),
      onSettled: () => setApplyingId(null),
    });
  };

  return (
    <Page>
      <PageHeader
        icon={Volleyball}
        eyebrow={t("eyebrow")}
        title={
          <span className="flex items-center gap-2.5">
            {t("title")}
            <Badge tone="warning">{t("experimental")}</Badge>
          </span>
        }
        subtitle={t("subtitle")}
        actions={
          <Button variant="ghost" onClick={() => void library.refetch()} loading={library.isFetching}>
            <RefreshCw />
            {t("rescan")}
          </Button>
        }
      />

      <div className="flex items-start gap-2.5 rounded-lg border border-warning/25 bg-warning/[0.06] p-3 text-[13px]">
        <AlertTriangle className="mt-0.5 size-4 shrink-0 text-warning" />
        <p className="text-fg-muted">{t("notice")}</p>
      </div>

      {pending > 0 ? (
        <GlassPanel className="flex flex-wrap items-center gap-3 p-3 text-[13px]">
          <FolderInput className="size-4 text-fg-subtle" />
          <p className="min-w-0 flex-1 text-fg-muted">{t("importBanner", { count: pending })}</p>
          <Button size="sm" variant="primary" onClick={() => setImporting(true)}>
            {t("import")}
          </Button>
        </GlassPanel>
      ) : null}

      {active ? (
        <GlassCard interactive={false} selected className="flex flex-wrap items-center gap-4 p-4">
          <PackPreview
            src={assetUrl(active.previewPath)}
            alt={active.displayName}
            className="size-16 rounded-md"
          />
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <p className="truncate font-semibold">{active.displayName}</p>
              <Badge tone={active.stale ? "warning" : "success"} dot pulse={!active.stale}>
                {active.stale ? t("stale") : t("inGame")}
              </Badge>
            </div>
            <p className="mt-0.5 text-xs text-fg-muted">{formatRelative(active.appliedAt)}</p>
          </div>
          <Button
            variant="danger"
            onClick={() => remove.mutate()}
            loading={remove.isPending}
            disabled={gameRunning}
          >
            <Trash2 />
            {t("remove")}
          </Button>
        </GlassCard>
      ) : null}

      <Section
        title={t("library.title")}
        description={t("library.count", { count: library.data?.length ?? 0 })}
      >
        {library.error ? <ErrorState error={library.error} onRetry={() => void library.refetch()} /> : null}
        {library.isLoading ? (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(190px,1fr))] gap-4">
            {Array.from({ length: 4 }, (_, i) => (
              // biome-ignore lint/suspicious/noArrayIndexKey: placeholders
              <Skeleton key={i} className="aspect-[4/5]" />
            ))}
          </div>
        ) : !library.data?.length ? (
          <GlassPanel>
            <EmptyState icon={Volleyball} title={t("library.empty")} description={t("library.emptyHint")} />
          </GlassPanel>
        ) : (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(190px,1fr))] gap-4">
            {library.data.map((pack) => (
              <BallCard
                key={pack.id}
                pack={pack}
                active={pack.id === active?.packId}
                blocked={blocked}
                applying={applyingId === pack.id}
                onApply={() => onApply(pack)}
              />
            ))}
          </div>
        )}
        {firstPack ? (
          <Button
            size="sm"
            variant="ghost"
            className="mt-3 self-start"
            onClick={() => void revealItemInDir(firstPack.templatePath).catch(notify.error)}
          >
            <FolderOpen />
            {t("openFolder")}
          </Button>
        ) : null}
      </Section>

      <BakkesImportDialog open={importing} onOpenChange={setImporting} focus="decals" />
    </Page>
  );
}
