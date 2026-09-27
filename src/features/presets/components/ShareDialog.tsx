import { Check, Copy } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { ErrorState, Skeleton } from "@/components/ui/feedback";
import { notify } from "@/components/ui/toast";
import type { Preset } from "../api";
import { useShareCode } from "../queries";

export function ShareDialog({ preset, onClose }: { preset: Preset | null; onClose: () => void }) {
  const { t } = useTranslation("presets");
  const { data: code, isLoading, error, refetch } = useShareCode(preset?.id ?? null);
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    if (!code) return;
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 1600);
    } catch (e) {
      notify.error(e);
    }
  };

  return (
    <Dialog open={preset !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        title={t("share.title", { name: preset?.name ?? "" })}
        description={t("share.description")}
        footer={
          <Button variant="primary" disabled={!code} onClick={() => void copy()}>
            {copied ? <Check /> : <Copy />}
            {copied ? t("common:actions.copied") : t("share.copy")}
          </Button>
        }
      >
        {isLoading ? (
          <Skeleton className="h-24" />
        ) : error ? (
          <ErrorState error={error} onRetry={() => void refetch()} />
        ) : (
          <pre
            data-selectable
            className="glass-inset max-h-48 overflow-auto whitespace-pre-wrap break-all rounded-md p-3 font-mono text-[11px] leading-relaxed text-fg-muted"
          >
            {code}
          </pre>
        )}
        <p className="mt-3 text-xs text-fg-subtle">{t("share.note")}</p>
      </DialogContent>
    </Dialog>
  );
}
