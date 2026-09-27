import { ArrowRight, Check, Info, Pencil, Wand2 } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import type { CatalogItem } from "../api";
import { itemLabel } from "../constants";
import { ItemThumb } from "./ItemThumb";

export type PickStep = "owned" | "wanted";

/** `update`: the owned item already has an active swap and the pick differs. */
export type ComposerMode = "new" | "update" | "unchanged";

interface SwapComposerProps {
  owned: CatalogItem | undefined;
  wanted: CatalogItem | undefined;
  step: PickStep;
  onStepChange: (step: PickStep) => void;
  paint: number;
  onPaintChange: (paint: number) => void;
  onApply: () => void;
  applying: boolean;
  mode: ComposerMode;
}

function PickCard({
  side,
  item,
  active,
  onClick,
}: {
  side: PickStep;
  item: CatalogItem | undefined;
  active: boolean;
  onClick: () => void;
}) {
  const { t } = useTranslation("items");
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "group relative flex min-w-0 flex-1 items-center gap-3 rounded-lg border p-2 text-left transition-colors duration-150",
        active
          ? "border-[var(--color-accent)] bg-[color-mix(in_oklab,var(--color-accent)_7%,transparent)]"
          : "border-line bg-white/[0.02] hover:border-line-strong",
      )}
    >
      <ItemThumb item={item} className="size-12 shrink-0" />
      <div className="min-w-0">
        <p className="text-xs text-fg-subtle">{t(`composer.${side}`)}</p>
        <AnimatePresence mode="wait" initial={false}>
          <motion.p
            key={item?.id ?? "none"}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.15 }}
            className={cn("truncate text-[13px] font-semibold", !item && "text-fg-subtle")}
          >
            {item ? itemLabel(item) : t(`composer.${side}Empty`)}
          </motion.p>
        </AnimatePresence>
        <p className="truncate text-[11px] text-fg-subtle">{item?.package ?? t(`composer.${side}Hint`)}</p>
      </div>
    </button>
  );
}

/**
 * Owned → wanted summary and the apply CTA.
 *
 * No paint choice: Rocket League applies paint at runtime from the owned
 * item (no painted package variants exist on disk), so a swapped item keeps
 * the paint of the item you own. `paint` props are kept for when painted
 * variants are found (see swap::rules).
 */
export function SwapComposer({
  owned,
  wanted,
  step,
  onStepChange,
  onApply,
  applying,
  mode,
}: SwapComposerProps) {
  const { t } = useTranslation("items");
  const ready = Boolean(owned && wanted) && mode !== "unchanged";
  return (
    <div className="glass flex flex-col gap-3 rounded-lg p-3">
      <div className="flex items-stretch gap-2">
        <PickCard side="owned" item={owned} active={step === "owned"} onClick={() => onStepChange("owned")} />
        <div className="grid w-8 shrink-0 place-items-center">
          <ArrowRight className={cn("size-4", ready ? "text-fg" : "text-fg-subtle")} />
        </div>
        <PickCard
          side="wanted"
          item={wanted}
          active={step === "wanted"}
          onClick={() => onStepChange("wanted")}
        />
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3 border-line border-t pt-3">
        <p className="flex max-w-md items-center gap-2 text-xs text-fg-muted">
          {mode === "new" ? <Info className="size-3.5 shrink-0" /> : <Pencil className="size-3.5 shrink-0" />}
          {mode === "new" ? t("paint.keptNote") : t("composer.editingNote")}
        </p>
        <Button variant="primary" disabled={!ready} loading={applying} onClick={onApply}>
          {mode === "unchanged" ? <Check /> : <Wand2 />}
          {t(`composer.${mode === "new" ? "apply" : mode}`)}
        </Button>
      </div>
    </div>
  );
}
