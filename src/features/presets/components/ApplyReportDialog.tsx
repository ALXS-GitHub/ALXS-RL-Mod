import { CheckCircle2, XCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import type { PresetApplyReport } from "../api";

export function ApplyReportDialog({
  report,
  onClose,
}: {
  report: PresetApplyReport | null;
  onClose: () => void;
}) {
  const { t } = useTranslation("presets");
  return (
    <Dialog open={report !== null} onOpenChange={(o) => !o && onClose()}>
      <DialogContent
        title={report?.ok ? t("report.okTitle") : t("report.partialTitle")}
        description={t("report.description")}
        size="sm"
        footer={
          <Button variant="primary" onClick={onClose}>
            {t("common:actions.close")}
          </Button>
        }
      >
        <ul className="flex flex-col gap-2">
          {report?.parts.map((p) => (
            <li key={p.part} className="rounded-[8px] border border-line bg-white/[0.02] p-3">
              <div className="flex items-center gap-2">
                {p.ok ? (
                  <CheckCircle2 className="size-4 text-success" />
                ) : (
                  <XCircle className="size-4 text-danger" />
                )}
                <span className="text-[13px] font-medium">{t(`parts.${p.part}`)}</span>
                {p.part === "swaps" ? (
                  <span className="ml-auto text-xs tabular-nums text-fg-muted">
                    {t("report.applied", { count: p.applied })}
                  </span>
                ) : null}
              </div>
              {p.errors.length > 0 ? (
                <ul className="mt-2 flex flex-col gap-1 text-xs text-fg-muted" data-selectable>
                  {p.errors.map((e) => (
                    <li key={e} className="break-words">
                      {e}
                    </li>
                  ))}
                </ul>
              ) : null}
            </li>
          ))}
        </ul>
      </DialogContent>
    </Dialog>
  );
}
