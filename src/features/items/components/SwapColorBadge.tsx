import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { usePaintLabel, usePaints } from "../queries";

interface SwapColorBadgeProps {
  paint?: number | null;
  color?: string | null;
  className?: string;
}

/**
 * Colour of a swap at a glance: a plain dot for an official paint, a dot
 * inside a rainbow ring for a custom colour. Nothing when uncoloured.
 */
export function SwapColorBadge({ paint, color, className }: SwapColorBadgeProps) {
  const { t } = useTranslation("items");
  const paints = usePaints();
  const paintLabel = usePaintLabel();
  if (color) {
    return (
      <span
        title={t("color.badge.custom", { hex: color.toUpperCase() })}
        className={cn("inline-grid size-3 shrink-0 place-items-center rounded-full", className)}
        style={{ background: "conic-gradient(#f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00)" }}
      >
        <span className="size-[7px] rounded-full" style={{ background: color }} />
      </span>
    );
  }
  const official = paint ? paints.data?.find((p) => p.id === paint) : undefined;
  if (!official) return null;
  return (
    <span
      title={t("color.badge.paint", { name: paintLabel(official.label) })}
      className={cn("inline-block size-2.5 shrink-0 rounded-full border border-white/30", className)}
      style={{ background: official.hex }}
    />
  );
}
