import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";

interface NameDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  description?: ReactNode;
  initialName?: string;
  confirmLabel: ReactNode;
  pending?: boolean;
  onConfirm: (name: string) => void;
}

/** Small "give it a name" dialog shared by capture, rename and save-random. */
export function NameDialog({
  open,
  onOpenChange,
  title,
  description,
  initialName = "",
  confirmLabel,
  pending,
  onConfirm,
}: NameDialogProps) {
  const { t } = useTranslation("presets");
  const [name, setName] = useState(initialName);
  useEffect(() => {
    if (open) setName(initialName);
  }, [open, initialName]);
  const valid = name.trim().length > 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        title={title}
        description={description}
        size="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              {t("common:actions.cancel")}
            </Button>
            <Button
              variant="primary"
              disabled={!valid}
              loading={pending}
              onClick={() => onConfirm(name.trim())}
            >
              {confirmLabel}
            </Button>
          </>
        }
      >
        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (valid) onConfirm(name.trim());
          }}
        >
          <label className="flex flex-col gap-1.5 text-xs text-fg-muted">
            {t("nameLabel")}
            <Input autoFocus maxLength={60} value={name} onChange={(e) => setName(e.target.value)} />
          </label>
        </form>
      </DialogContent>
    </Dialog>
  );
}
