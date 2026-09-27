import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { Download, RotateCw, Sparkles } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { notify } from "@/components/ui/toast";
import { formatBytes } from "@/lib/format";
import { useUi } from "@/stores/ui";

type Phase =
  | { kind: "available"; update: Update }
  | { kind: "downloading"; update: Update; downloaded: number; total: number }
  | { kind: "installing"; update: Update }
  | { kind: "installed"; update: Update };

/**
 * Checks GitHub Releases for a newer signed build: once at startup, and
 * again when Settings asks (`requestUpdateCheck`). The updater plugin is only
 * registered in release builds; in dev `check()` throws and is ignored.
 */
export function UpdateDialog() {
  const { t } = useTranslation();
  const [phase, setPhase] = useState<Phase | null>(null);
  const request = useUi((s) => s.updateCheck);
  const startup = useRef(false);

  const runCheck = useCallback(
    async (manual: boolean) => {
      try {
        const update = await check();
        if (update) setPhase({ kind: "available", update });
        else if (manual) notify.success(t("update.upToDate"));
      } catch (err) {
        if (manual) notify.error(err);
      }
    },
    [t],
  );

  useEffect(() => {
    if (startup.current) return;
    startup.current = true;
    void runCheck(false);
  }, [runCheck]);

  useEffect(() => {
    if (request > 0) void runCheck(true);
  }, [request, runCheck]);

  const install = async () => {
    if (phase?.kind !== "available") return;
    const update = phase.update;
    let total = 0;
    let downloaded = 0;
    setPhase({ kind: "downloading", update, downloaded, total });
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
          setPhase({ kind: "downloading", update, downloaded: 0, total });
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          setPhase({ kind: "downloading", update, downloaded, total });
        } else {
          setPhase({ kind: "installing", update });
        }
      });
      setPhase({ kind: "installed", update });
    } catch (err) {
      notify.error(err);
      setPhase(null);
    }
  };

  if (!phase) return null;
  const notes = phase.update.body?.trim();
  const busy = phase.kind === "downloading" || phase.kind === "installing";

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && setPhase(null)}>
      <DialogContent
        size="md"
        title={
          <span className="flex items-center gap-2">
            <Sparkles className="size-4 text-[var(--color-accent)]" />
            {t("update.title")}
          </span>
        }
        description={t("update.versions", { from: phase.update.currentVersion, to: phase.update.version })}
        onPointerDownOutside={(e) => busy && e.preventDefault()}
        footer={
          phase.kind === "available" ? (
            <>
              <Button variant="ghost" onClick={() => setPhase(null)}>
                {t("update.later")}
              </Button>
              <Button variant="primary" onClick={() => void install()}>
                <Download />
                {t("update.install")}
              </Button>
            </>
          ) : phase.kind === "installed" ? (
            <Button variant="primary" onClick={() => void relaunch().catch(notify.error)}>
              <RotateCw />
              {t("update.restart")}
            </Button>
          ) : null
        }
      >
        {phase.kind === "available" && notes ? (
          <div className="max-h-60 overflow-y-auto rounded-md border border-line bg-white/[0.02] p-3 text-xs whitespace-pre-wrap text-fg-muted">
            {notes}
          </div>
        ) : null}
        {phase.kind === "downloading" ? (
          <div className="flex flex-col gap-2">
            <div className="h-1.5 overflow-hidden rounded-full bg-white/[0.08]">
              <div
                className="h-full bg-[var(--color-accent)] transition-[width]"
                style={{ width: phase.total > 0 ? `${(100 * phase.downloaded) / phase.total}%` : "10%" }}
              />
            </div>
            <p className="flex justify-between text-xs text-fg-subtle">
              <span>{t("update.downloading")}</span>
              <span className="tabular-nums">
                {formatBytes(phase.downloaded)}
                {phase.total > 0 ? ` / ${formatBytes(phase.total)}` : ""}
              </span>
            </p>
          </div>
        ) : null}
        {phase.kind === "installing" ? (
          <p className="flex items-center gap-2 text-[13px] text-fg-muted">
            <RotateCw className="size-4 animate-spin" />
            {t("update.installing")}
          </p>
        ) : null}
        {phase.kind === "installed" ? (
          <p className="flex items-center gap-2 text-[13px] text-fg-muted">
            <Badge tone="success" dot>
              {phase.update.version}
            </Badge>
            {t("update.installed")}
          </p>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
