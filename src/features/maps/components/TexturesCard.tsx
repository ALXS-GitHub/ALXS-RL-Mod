import { Check, Download, Layers3, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { GlassPanel } from "@/components/ui/glass";
import { notify } from "@/components/ui/toast";
import { useGameStatus } from "@/lib/game";
import { TEXTURES_REMOTE_ID } from "../api";
import { useDownloadProgress, useInstallTextures, useRemoveTextures, useTexturesStatus } from "../queries";

/** Install / remove buttons for the workshop textures pack (shared with the multi page). */
export function TexturesActions({ compact = false }: { compact?: boolean }) {
  const { t } = useTranslation("maps");
  const { data: status } = useTexturesStatus();
  const { data: game } = useGameStatus();
  const install = useInstallTextures();
  const remove = useRemoveTextures();
  const progress = useDownloadProgress()[`bakkesPlugins:${TEXTURES_REMOTE_ID}`];
  const running = game?.running.running ?? false;
  const percent = progress?.total ? Math.round((progress.downloaded / progress.total) * 100) : null;

  if (!status) return null;
  if (status.installed) {
    return status.managed && !compact ? (
      <Button
        size="sm"
        variant="ghost"
        loading={remove.isPending}
        disabled={running}
        title={running ? t("textures.closeGame") : undefined}
        onClick={() => remove.mutate(undefined, { onSuccess: () => notify.success(t("textures.removed")) })}
      >
        <Trash2 />
        {t("textures.remove")}
      </Button>
    ) : null;
  }
  return (
    <Button
      size="sm"
      variant="secondary"
      loading={install.isPending}
      disabled={running}
      title={running ? t("textures.closeGame") : undefined}
      onClick={() => install.mutate(undefined, { onSuccess: () => notify.success(t("textures.installed")) })}
    >
      <Download />
      {install.isPending && percent !== null ? t("textures.downloading", { percent }) : t("textures.install")}
    </Button>
  );
}

/** Text for the pack's state. */
export function useTexturesLabel(): string | null {
  const { t } = useTranslation("maps");
  const { data: s } = useTexturesStatus();
  if (!s) return null;
  if (s.installed) return t("textures.ok");
  if (s.present > 0) return t("textures.partial", { present: s.present, total: s.total });
  return t("textures.missing");
}

export function TexturesCard() {
  const { t } = useTranslation("maps");
  const { data: s } = useTexturesStatus();
  const label = useTexturesLabel();
  if (!s) return null;
  return (
    <GlassPanel className="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
      <div className="flex min-w-0 items-start gap-2.5">
        {s.installed ? (
          <Check className="mt-0.5 size-4 shrink-0 text-success" />
        ) : (
          <Layers3 className="mt-0.5 size-4 shrink-0 text-fg-subtle" />
        )}
        <div className="min-w-0">
          <p className="text-[13px] font-medium">{t("textures.title")}</p>
          <p className="mt-0.5 text-xs text-fg-muted">
            {label} {s.installed ? null : t("textures.why")}
          </p>
        </div>
      </div>
      <TexturesActions />
    </GlassPanel>
  );
}
