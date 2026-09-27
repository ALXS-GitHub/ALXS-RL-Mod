import { RotateCcw } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Field } from "@/components/ui/layout";
import { notify } from "@/components/ui/toast";
import { useGameStatus } from "@/lib/game";
import { useRestoreStock } from "../api";

/** Confirmation dialog of "restore stock game" (settings, active mods panel). */
export function RestoreStockDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation("settings");
  const restore = useRestoreStock();

  const confirm = () =>
    restore.mutate(undefined, {
      onSuccess: (r) => {
        onOpenChange(false);
        if (r.failures.length > 0) {
          for (const f of r.failures)
            notify.error({ kind: "Internal", message: `${f.feature}: ${f.message}` });
          return;
        }
        notify.success(t("stock.done"), t("stock.doneDetail", { count: r.swaps + r.leftovers }));
      },
      onError: notify.error,
    });

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        size="sm"
        title={t("stock.confirmTitle")}
        description={t("stock.confirmBody")}
        footer={
          <>
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              {t("common:actions.cancel")}
            </Button>
            <Button variant="danger" loading={restore.isPending} onClick={confirm}>
              {t("stock.action")}
            </Button>
          </>
        }
      >
        <ul className="list-disc space-y-1 pl-5 text-[13px] text-fg-muted">
          <li>{t("stock.listSwaps")}</li>
          <li>{t("stock.listPalette")}</li>
          <li>{t("stock.listDecal")}</li>
          <li>{t("stock.listMap")}</li>
        </ul>
        <p className="mt-3 text-xs text-fg-subtle">{t("stock.kept")}</p>
      </DialogContent>
    </Dialog>
  );
}

/** "Restore stock game": one confirmed click that removes every modification. */
export function RestoreStockField() {
  const { t } = useTranslation("settings");
  const [open, setOpen] = useState(false);
  const running = useGameStatus().data?.running.running ?? false;

  return (
    <>
      <Field label={t("stock.title")} description={running ? t("stock.gameRunning") : t("stock.hint")}>
        <Button size="sm" variant="outline" disabled={running} onClick={() => setOpen(true)}>
          <RotateCcw />
          {t("stock.action")}
        </Button>
      </Field>
      <RestoreStockDialog open={open} onOpenChange={setOpen} />
    </>
  );
}
