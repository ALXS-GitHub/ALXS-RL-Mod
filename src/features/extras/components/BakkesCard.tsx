import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { FolderInput, FolderOpen, Plug, ShieldAlert } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ErrorState, Skeleton } from "@/components/ui/feedback";
import { GlassPanel } from "@/components/ui/glass";
import { notify } from "@/components/ui/toast";
import { formatBytes } from "@/lib/format";
import { useBakkesStatus } from "../queries";
import { BakkesImportDialog } from "./BakkesImportDialog";

export function BakkesCard() {
  const { t } = useTranslation("extras");
  const status = useBakkesStatus();
  const [importing, setImporting] = useState(false);
  const s = status.data;

  return (
    <GlassPanel className="flex flex-col gap-3 p-4">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2.5">
          <Plug className="size-4 text-fg-subtle" />
          <h2 className="text-sm font-semibold">BakkesMod</h2>
        </div>
        {s ? (
          <Badge tone={s.installed ? "success" : "neutral"} dot>
            {s.installed ? t("bakkes.detected") : t("bakkes.notDetected")}
          </Badge>
        ) : null}
      </div>

      <div className="flex items-start gap-2.5 rounded-[8px] border border-warning/25 bg-warning/[0.06] p-2.5 text-xs text-fg-muted">
        <ShieldAlert className="mt-0.5 size-3.5 shrink-0 text-warning" />
        <p>{t("bakkes.eacNote")}</p>
      </div>

      {status.isError ? (
        <ErrorState error={status.error} onRetry={() => void status.refetch()} />
      ) : !s ? (
        <Skeleton className="h-32" />
      ) : !s.installed ? (
        <p className="text-[13px] text-fg-muted">{t("bakkes.absent")}</p>
      ) : (
        <div className="flex flex-col gap-4">
          <div>
            <p className="mb-2 text-xs text-fg-subtle">{t("bakkes.plugins", { count: s.plugins.length })}</p>
            {s.plugins.length ? (
              <div className="flex flex-wrap gap-1.5">
                {s.plugins.map((p) => (
                  <Badge key={p.name} title={formatBytes(p.sizeBytes)}>
                    {p.name}
                  </Badge>
                ))}
              </div>
            ) : (
              <p className="text-[13px] text-fg-muted">{t("bakkes.noPlugins")}</p>
            )}
          </div>
          {s.alphaConsoleData ? <p className="text-xs text-fg-muted">{t("bakkes.alphaConsole")}</p> : null}
          <div className="flex flex-wrap gap-2">
            {s.workshopDir || s.alphaConsoleData ? (
              <Button size="sm" variant="primary" onClick={() => setImporting(true)}>
                <FolderInput />
                {t("import.open")}
              </Button>
            ) : null}
            {s.dataDir ? (
              <Button
                size="sm"
                variant="secondary"
                onClick={() => s.dataDir && void revealItemInDir(s.dataDir).catch(notify.error)}
              >
                <FolderOpen />
                {t("bakkes.openData")}
              </Button>
            ) : null}
          </div>
        </div>
      )}
      <BakkesImportDialog open={importing} onOpenChange={setImporting} />
    </GlassPanel>
  );
}
