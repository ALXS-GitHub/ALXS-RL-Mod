import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import type { Rgb } from "../api";
import { toHex } from "../colors";

interface SwatchGridProps {
  colors: readonly Rgb[];
  columns: number;
  rows: number;
  selected: number | null;
  onSelect: (index: number) => void;
  /** Slots that differ from stock get a subtle marker. */
  stock?: readonly Rgb[];
}

const same = (a?: Rgb, b?: Rgb) => !!a && !!b && a.r === b.r && a.g === b.g && a.b === b.b;

/** The game's picker grid, row-major (row = shade, column = hue). */
export function SwatchGrid({ colors, columns, rows, selected, onSelect, stock }: SwatchGridProps) {
  const { t } = useTranslation("palette");
  return (
    <div
      role="grid"
      className="grid gap-1"
      style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}
    >
      {Array.from({ length: rows * columns }, (_, i) => {
        const c = colors[i] ?? { r: 0, g: 0, b: 0 };
        const hex = toHex(c);
        const active = selected === i;
        const edited = stock ? !same(c, stock[i]) : false;
        return (
          <motion.button
            // biome-ignore lint/suspicious/noArrayIndexKey: slots are positional
            key={i}
            type="button"
            role="gridcell"
            aria-selected={active}
            aria-label={t("editor.slot", { index: i + 1, hex })}
            onClick={() => onSelect(i)}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.15 }}
            className={cn(
              "relative aspect-square rounded-[5px] outline-none shadow-[inset_0_0_0_1px_rgb(255_255_255/0.08)] hover:shadow-[inset_0_0_0_1px_rgb(255_255_255/0.35)]",
              active && "ring-2 ring-white ring-offset-1 ring-offset-[var(--color-bg)]",
            )}
            style={{ background: hex }}
          >
            {edited ? (
              <span className="absolute top-1 right-1 size-1.5 rounded-full bg-white/85 ring-1 ring-black/40" />
            ) : null}
          </motion.button>
        );
      })}
    </div>
  );
}
