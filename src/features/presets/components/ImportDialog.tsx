import { ClipboardPaste, Download } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogTrigger } from "@/components/ui/dialog";
import { useImportPreset } from "../queries";

const PREFIX = "ALXS1:";

export function ImportDialog() {
  const { t } = useTranslation("presets");
  const [open, setOpen] = useState(false);
  const [code, setCode] = useState("");
  const importPreset = useImportPreset();
  const looksValid = code.trim().startsWith(PREFIX) && code.trim().length > PREFIX.length + 8;

  const paste = async () => {
    try {
      setCode(await navigator.clipboard.readText());
    } catch {
      // Clipboard read may be refused; the user can still paste manually.
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        setOpen(o);
        if (!o) setCode("");
      }}
    >
      <DialogTrigger asChild>
        <Button variant="secondary">
          <Download />
          {t("import.open")}
        </Button>
      </DialogTrigger>
      <DialogContent
        title={t("import.title")}
        description={t("import.description")}
        footer={
          <>
            <Button variant="ghost" onClick={() => void paste()}>
              <ClipboardPaste />
              {t("import.paste")}
            </Button>
            <Button
              variant="primary"
              disabled={!looksValid}
              loading={importPreset.isPending}
              onClick={() => importPreset.mutate(code.trim(), { onSuccess: () => setOpen(false) })}
            >
              {t("import.confirm")}
            </Button>
          </>
        }
      >
        <textarea
          value={code}
          onChange={(e) => setCode(e.target.value)}
          placeholder={`${PREFIX}…`}
          spellCheck={false}
          rows={6}
          className="glass-inset w-full resize-none rounded-[7px] p-2.5 font-mono text-xs text-fg outline-none placeholder:text-fg-subtle focus:border-[var(--color-accent)]"
        />
        {code && !looksValid ? <p className="mt-2 text-xs text-warning">{t("import.invalid")}</p> : null}
      </DialogContent>
    </Dialog>
  );
}
