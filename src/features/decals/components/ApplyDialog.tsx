import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { AlertTriangle, CheckCircle2, FolderOpen, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Segmented } from "@/components/ui/segmented";
import { Select } from "@/components/ui/select";
import type { DecalPack, TargetSlot } from "../api";
import { assetUrl, PackPreview } from "./PackCard";

interface ApplyDialogProps {
  /** Pack to show; `null` closes the dialog. */
  pack: DecalPack | null;
  targets: readonly TargetSlot[];
  blocked: string | null;
  applying: boolean;
  onApply: (pack: DecalPack, target: string) => void;
  onClose: () => void;
}

/** Details of a pack and where to apply it, opened from its card. */
export function ApplyDialog({ pack, targets, blocked, applying, onApply, onClose }: ApplyDialogProps) {
  const { t } = useTranslation("decals");
  const [view, setView] = useState<"art" | "mask">("mask");
  const slots = pack ? targets.filter((s) => s.bodyId === pack.bodyId) : [];
  const [target, setTarget] = useState<string | undefined>(slots[0]?.key);

  // Reset the choices when another pack is opened.
  // biome-ignore lint/correctness/useExhaustiveDependencies: keyed on the pack id only
  useEffect(() => {
    setTarget(slots[0]?.key);
    setView(pack?.hybrid || pack?.universal || !pack?.maskPreviewPath ? "art" : "mask");
  }, [pack?.id]);

  const slot = slots.find((s) => s.key === target);
  const reason = !pack
    ? null
    : !pack.maskPath && !pack.universal
      ? t("panel.noMask")
      : !pack.supported
        ? t("panel.bodyUnsupported", { body: pack.bodyFolder })
        : blocked;

  return (
    <Dialog open={pack !== null} onOpenChange={(open) => !open && onClose()}>
      {pack ? (
        <DialogContent
          size="lg"
          title={
            <span className="flex items-center gap-2">
              {pack.displayName}
              {pack.hybrid ? <Badge tone="accent">{t("pack.hybrid")}</Badge> : null}
              {pack.universal ? <Badge tone="accent">{t("pack.universal")}</Badge> : null}
            </span>
          }
          description={pack.group || pack.packName}
          footer={
            <>
              <Button variant="ghost" onClick={() => void revealItemInDir(pack.templatePath)}>
                <FolderOpen />
                {t("panel.openFolder")}
              </Button>
              <Button
                variant="primary"
                disabled={Boolean(reason) || !target}
                loading={applying}
                onClick={() => target && onApply(pack, target)}
              >
                <Sparkles />
                {t("panel.apply")}
              </Button>
            </>
          }
        >
          <div className="grid gap-5 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
            <div className="relative overflow-hidden rounded-lg border border-line">
              <PackPreview
                src={assetUrl(view === "mask" ? pack.maskPreviewPath : pack.previewPath)}
                alt={pack.displayName}
                className="aspect-square"
              />
              <div
                className={pack.universal ? "hidden" : "absolute inset-x-0 bottom-0 flex justify-center p-3"}
              >
                <Segmented
                  size="sm"
                  value={view}
                  onValueChange={setView}
                  options={[
                    { value: "mask", label: t("panel.viewMask") },
                    { value: "art", label: t("panel.viewArt") },
                  ]}
                  className="bg-black/50 backdrop-blur-md"
                />
              </div>
            </div>

            <div className="flex flex-col gap-4">
              <div className="flex flex-wrap gap-1.5">
                {pack.body.map((tex) => (
                  <Badge key={tex.role} tone={tex.exists ? "neutral" : "danger"}>
                    {tex.role}
                  </Badge>
                ))}
              </div>

              <p className="text-xs text-fg-subtle">
                {t(
                  pack.universal ? "panel.universalNote" : pack.hybrid ? "panel.hybridNote" : "panel.artNote",
                )}
              </p>

              {slots.length > 0 ? (
                <div className="space-y-2">
                  <p className="text-xs text-fg-subtle">{t("panel.slot")}</p>
                  {slots.length > 3 ? (
                    <Select
                      value={target ?? ""}
                      onValueChange={setTarget}
                      options={slots.map((s) => ({ value: s.key, label: s.label }))}
                    />
                  ) : (
                    <Segmented
                      value={target ?? ""}
                      onValueChange={setTarget}
                      options={slots.map((s) => ({ value: s.key, label: s.label }))}
                    />
                  )}
                  {slot ? (
                    <p className="flex items-start gap-2 text-xs text-fg-muted">
                      <CheckCircle2 className="mt-0.5 size-3.5 shrink-0 text-success" />
                      {t("panel.equipHint", { decal: slot.label, body: slot.bodyName })}
                    </p>
                  ) : null}
                </div>
              ) : null}

              {reason ? (
                <p className="flex items-start gap-2 rounded-[8px] border border-warning/25 bg-warning/[0.06] p-2.5 text-xs text-fg">
                  <AlertTriangle className="mt-0.5 size-3.5 shrink-0 text-warning" />
                  {reason}
                </p>
              ) : null}
            </div>
          </div>
        </DialogContent>
      ) : null}
    </Dialog>
  );
}
