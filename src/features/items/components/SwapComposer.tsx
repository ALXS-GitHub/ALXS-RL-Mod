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
  /** Experimental recolour hue (degrees), `null` = original colours. */
  tint: number | null;
  onTintChange: (tint: number | null) => void;
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

/** Hues offered for the experimental recolour (degrees). */
const TINTS = [
  { key: "red", hue: 0 },
  { key: "orange", hue: 25 },
  { key: "yellow", hue: 55 },
  { key: "green", hue: 120 },
  { key: "cyan", hue: 185 },
  { key: "blue", hue: 220 },
  { key: "purple", hue: 275 },
  { key: "pink", hue: 320 },
] as const;

function TintPicker({ value, onChange }: { value: number | null; onChange: (tint: number | null) => void }) {
  const { t } = useTranslation("items");
  const swatch = "size-6 rounded-full border transition-shadow duration-150";
  const ring = "shadow-[0_0_0_2px_var(--color-bg),0_0_0_3.5px_var(--color-accent)]";
  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="mr-1 text-xs text-fg-subtle">{t("tint.label")}</span>
      <button
        type="button"
        title={t("tint.original")}
        aria-label={t("tint.original")}
        aria-pressed={value === null}
        onClick={() => onChange(null)}
        className={cn(
          swatch,
          "grid place-items-center border-line-strong bg-white/[0.04]",
          value === null && ring,
        )}
      >
        <span className="h-px w-3.5 rotate-45 bg-fg-subtle" />
      </button>
      {TINTS.map(({ key, hue }) => (
        <button
          key={key}
          type="button"
          title={t(`tint.${key}`)}
          aria-label={t(`tint.${key}`)}
          aria-pressed={value === hue}
          onClick={() => onChange(hue)}
          className={cn(swatch, "border-white/10", value === hue && ring)}
          style={{ background: `hsl(${hue} 85% 55%)` }}
        />
      ))}
    </div>
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
  tint,
  onTintChange,
  onApply,
  applying,
  mode,
}: SwapComposerProps) {
  const { t } = useTranslation("items");
  // An item can replace itself only to be recoloured.
  const recolourOnly = Boolean(owned && wanted && owned.id === wanted.id);
  const ready = Boolean(owned && wanted) && mode !== "unchanged" && (!recolourOnly || tint !== null);
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
      {wanted ? (
        <div className="flex flex-col gap-1.5 border-line border-t pt-3">
          <TintPicker value={tint} onChange={onTintChange} />
          <p className="text-[11px] text-fg-subtle">{t(recolourOnly ? "tint.selfHint" : "tint.hint")}</p>
        </div>
      ) : null}
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
