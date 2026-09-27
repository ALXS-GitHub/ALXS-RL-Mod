import { useNavigate } from "@tanstack/react-router";
import { FolderInput, Map as MapIcon, Sparkles, Volleyball } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import { notify } from "@/components/ui/toast";
import { useBallStatus, useImportBalls } from "@/features/ball/queries";
import { useDecalStatus, useImportAlphaConsole } from "@/features/decals/queries";
import { useImportMaps } from "@/features/maps/queries";
import { useBakkesStatus } from "../queries";

interface BakkesImportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Pre-select what the dialog was opened for. */
  focus?: "all" | "decals";
}

function Option({
  icon: Icon,
  title,
  detail,
  checked,
  onCheckedChange,
  disabled,
  children,
}: {
  icon: typeof MapIcon;
  title: string;
  detail: string;
  checked: boolean;
  onCheckedChange: (v: boolean) => void;
  disabled?: boolean;
  children?: React.ReactNode;
}) {
  return (
    <div className="rounded-lg border border-line p-3">
      <label className="flex cursor-pointer items-start gap-3">
        <Icon className="mt-0.5 size-4 shrink-0 text-fg-subtle" />
        <span className="min-w-0 flex-1">
          <span className="block text-[13px] font-medium">{title}</span>
          <span className="block text-xs text-fg-muted">{detail}</span>
        </span>
        <Switch checked={checked} onCheckedChange={onCheckedChange} disabled={disabled} />
      </label>
      {children}
    </div>
  );
}

/** Imports what BakkesMod left behind: Workshop maps and AlphaConsole decal packs. */
export function BakkesImportDialog({ open, onOpenChange, focus = "all" }: BakkesImportDialogProps) {
  const { t } = useTranslation("extras");
  const bakkes = useBakkesStatus();
  const decals = useDecalStatus();
  const importMaps = useImportMaps();
  const importDecals = useImportAlphaConsole();
  const ballStatus = useBallStatus();
  const importBalls = useImportBalls();
  const navigate = useNavigate();

  const workshopDir = bakkes.data?.workshopDir ?? null;
  const ac = decals.data?.alphaConsole ?? null;
  const [maps, setMaps] = useState(false);
  const [packs, setPacks] = useState(false);
  const [convert, setConvert] = useState(true);
  const acBalls = ballStatus.data?.alphaConsole ?? null;
  const [balls, setBalls] = useState(false);

  useEffect(() => {
    if (!open) return;
    setMaps(focus === "all" && Boolean(workshopDir));
    setPacks(Boolean(ac && ac.notImported > 0));
    setConvert(true);
    setBalls(Boolean(acBalls && acBalls.notImported > 0));
  }, [open, focus, workshopDir, ac, acBalls]);

  const busy = importMaps.isPending || importDecals.isPending || importBalls.isPending;

  const run = async () => {
    try {
      if (maps && workshopDir) {
        const created = await importMaps.mutateAsync([workshopDir]);
        notify.success(t("bakkes.imported", { count: created.length }));
      }
      if (packs) {
        const r = await importDecals.mutateAsync(convert);
        notify.success(
          t("import.decalsDone", { count: r.imported }),
          [
            convert ? t("import.converted", { count: r.converted }) : null,
            r.already ? t("import.already", { count: r.already }) : null,
            r.failed.length ? t("import.failed", { count: r.failed.length }) : null,
          ]
            .filter(Boolean)
            .join(" · ") || undefined,
        );
      }
      if (balls) {
        const r = await importBalls.mutateAsync();
        notify.success(t("import.ballsDone", { count: r.imported }));
      }
      onOpenChange(false);
      if (packs) void navigate({ to: "/decals" });
      else if (balls) void navigate({ to: "/ball" });
      else if (maps) void navigate({ to: "/maps" });
    } catch {
      // Errors are toasted by the mutations.
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        title={t("import.title")}
        description={t("import.description")}
        footer={
          <>
            <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={busy}>
              {t("common:actions.cancel")}
            </Button>
            <Button
              variant="primary"
              loading={busy}
              disabled={!maps && !packs && !balls}
              onClick={() => void run()}
            >
              <FolderInput />
              {t("import.run")}
            </Button>
          </>
        }
      >
        <div className="flex flex-col gap-3">
          <Option
            icon={MapIcon}
            title={t("import.maps")}
            detail={workshopDir ?? t("import.noWorkshop")}
            checked={maps}
            onCheckedChange={setMaps}
            disabled={!workshopDir}
          />
          <Option
            icon={Sparkles}
            title={t("import.decals")}
            detail={
              ac
                ? t("import.decalsDetail", { count: ac.notImported, total: ac.packCount })
                : t("import.noAlphaConsole")
            }
            checked={packs}
            onCheckedChange={setPacks}
            disabled={!ac || ac.notImported === 0}
          >
            {packs ? (
              <label className="mt-3 flex cursor-pointer items-start gap-3 border-line border-t pt-3">
                <span className="min-w-0 flex-1">
                  <span className="block text-[13px] font-medium">{t("import.convert")}</span>
                  <span className="block text-xs text-fg-muted">{t("import.convertDetail")}</span>
                </span>
                <Switch checked={convert} onCheckedChange={setConvert} />
              </label>
            ) : null}
          </Option>
          <Option
            icon={Volleyball}
            title={t("import.balls")}
            detail={
              acBalls
                ? t("import.decalsDetail", { count: acBalls.notImported, total: acBalls.packCount })
                : t("import.noAlphaConsole")
            }
            checked={balls}
            onCheckedChange={setBalls}
            disabled={!acBalls || acBalls.notImported === 0}
          />
          <p className="text-xs text-fg-subtle">{t("import.untouched")}</p>
        </div>
      </DialogContent>
    </Dialog>
  );
}
