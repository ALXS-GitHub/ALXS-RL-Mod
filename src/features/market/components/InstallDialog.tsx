import { CloudDownload } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import type { MarketItem } from "../api";

interface InstallDialogProps {
  item: MarketItem | null;
  busy: boolean;
  onCancel: () => void;
  onConfirm: (convert: boolean) => void;
}

/**
 * Asked before every AlphaConsole decal install: convert the pack to the
 * app's real-colour format (default) or keep it as is.
 */
export function InstallDialog({ item, busy, onCancel, onConfirm }: InstallDialogProps) {
  const { t } = useTranslation("market");
  const [convert, setConvert] = useState(true);

  useEffect(() => {
    if (item) setConvert(true);
  }, [item]);

  return (
    <Dialog open={item !== null} onOpenChange={(open) => (!open && !busy ? onCancel() : undefined)}>
      <DialogContent
        size="sm"
        title={t("install.title", { name: item?.title ?? "" })}
        description={t("install.description")}
        footer={
          <>
            <Button variant="ghost" onClick={onCancel} disabled={busy}>
              {t("install.cancel")}
            </Button>
            <Button variant="primary" loading={busy} onClick={() => onConfirm(convert)}>
              <CloudDownload />
              {t("install.confirm")}
            </Button>
          </>
        }
      >
        <label className="flex cursor-pointer items-start gap-3">
          <span className="min-w-0 flex-1">
            <span className="block text-[13px] font-medium">{t("install.convert")}</span>
            <span className="block text-xs text-fg-muted">{t("install.convertDetail")}</span>
          </span>
          <Switch checked={convert} onCheckedChange={setConvert} />
        </label>
      </DialogContent>
    </Dialog>
  );
}
