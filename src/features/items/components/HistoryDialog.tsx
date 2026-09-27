import { CheckCircle2, History, RefreshCw, TriangleAlert, Undo2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogTrigger } from "@/components/ui/dialog";
import { EmptyState, ErrorState, Skeleton } from "@/components/ui/feedback";
import { cn } from "@/lib/cn";
import { formatDateTime } from "@/lib/format";
import type { SwapEvent } from "../api";
import { useSwapHistory } from "../queries";

const KIND_STYLE: Record<SwapEvent["kind"], { icon: typeof CheckCircle2; className: string }> = {
  applied: { icon: CheckCircle2, className: "text-success" },
  restored: { icon: Undo2, className: "text-info" },
  reapplied: { icon: RefreshCw, className: "text-[var(--color-accent)]" },
  dropped: { icon: TriangleAlert, className: "text-warning" },
};

export function HistoryDialog() {
  const { t } = useTranslation("items");
  const [open, setOpen] = useState(false);
  const { data, isLoading, error, refetch } = useSwapHistory(open);

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="secondary">
          <History />
          {t("history.open")}
        </Button>
      </DialogTrigger>
      <DialogContent title={t("history.title")} description={t("history.description")} size="md">
        {isLoading ? (
          <div className="flex flex-col gap-2">
            {[0, 1, 2, 3].map((i) => (
              <Skeleton key={i} className="h-12" />
            ))}
          </div>
        ) : error ? (
          <ErrorState error={error} onRetry={() => void refetch()} />
        ) : !data?.length ? (
          <EmptyState icon={History} title={t("history.empty")} />
        ) : (
          <ol className="relative flex flex-col gap-1 border-line border-l pl-5">
            {data.map((ev) => {
              const style = KIND_STYLE[ev.kind];
              const Icon = style.icon;
              return (
                <li key={`${ev.swapId}-${ev.at}-${ev.kind}`} className="relative py-2">
                  <span className="absolute top-3 -left-[27px] grid size-3.5 place-items-center rounded-full bg-bg">
                    <Icon className={cn("size-3.5", style.className)} />
                  </span>
                  <p className="text-sm">
                    <span className={cn("font-medium", style.className)}>{t(`history.kind.${ev.kind}`)}</span>{" "}
                    <span className="text-fg">{ev.wantedLabel}</span>{" "}
                    <span className="text-fg-muted">{t("history.on", { owned: ev.ownedLabel })}</span>
                  </p>
                  <p className="text-[11px] text-fg-subtle">
                    {t(`slots.${ev.slot}`)} · {formatDateTime(ev.at)}
                  </p>
                  {ev.detail ? (
                    <p className="mt-1 text-xs text-warning" data-selectable>
                      {ev.detail}
                    </p>
                  ) : null}
                </li>
              );
            })}
          </ol>
        )}
      </DialogContent>
    </Dialog>
  );
}
