import { Ban } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Tooltip } from "@/components/ui/tooltip";
import { cn } from "@/lib/cn";
import { PAINTS } from "../constants";

interface PaintPickerProps {
  value: number;
  onChange: (paint: number) => void;
}

/** The 12 in-game paints. Applies only when the item ships a painted variant. */
export function PaintPicker({ value, onChange }: PaintPickerProps) {
  const { t } = useTranslation("items");
  return (
    <div role="radiogroup" aria-label={t("paint.title")} className="flex flex-wrap items-center gap-1.5">
      {PAINTS.map((paint) => {
        const selected = paint.id === value;
        return (
          <Tooltip key={paint.id} content={t(`paint.${paint.key}`)}>
            <button
              type="button"
              role="radio"
              aria-checked={selected}
              aria-label={t(`paint.${paint.key}`)}
              onClick={() => onChange(paint.id)}
              className={cn(
                "relative grid size-5 place-items-center rounded-full border transition-colors duration-150",
                selected
                  ? "border-white ring-1 ring-[var(--color-accent)] ring-offset-1 ring-offset-[var(--color-bg)]"
                  : "border-white/20",
              )}
              style={{ background: paint.id === 0 ? "rgb(255 255 255 / 0.06)" : paint.hex }}
            >
              {paint.id === 0 ? <Ban className="size-3 text-fg-subtle" /> : null}
            </button>
          </Tooltip>
        );
      })}
    </div>
  );
}
