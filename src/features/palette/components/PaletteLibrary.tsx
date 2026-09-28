import { Copy, Plus, Trash2 } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/feedback";
import { GlassCard } from "@/components/ui/glass";
import { Tooltip } from "@/components/ui/tooltip";
import { formatRelative } from "@/lib/format";
import { type Palette, STOCK_ID } from "../api";
import { ACCENT_COLUMNS, PICKER_ROWS, PRIMARY_COLUMNS, toHex } from "../colors";

interface PaletteLibraryProps {
  palettes: readonly Palette[] | undefined;
  stock: Palette | undefined;
  selectedId: string | null;
  activeId: string | null;
  onSelect: (p: Palette) => void;
  onNew: () => void;
  onDuplicate: (p: Palette) => void;
  onDelete: (p: Palette) => void;
}

/**
 * Tiny 3-band preview: blue primary · orange primary · accent. Grids are
 * row-major (shade rows × hue columns): each band shows every column on
 * the middle shade row, the hues as the in-game picker shows them.
 */
function Strip({ p }: { p: Palette }) {
  const middle = Math.floor(PICKER_ROWS / 2);
  const hues = (arr: Palette["accent"], columns: number) =>
    Array.from({ length: columns }, (_, c) => arr[middle * columns + c] ?? arr[c] ?? { r: 0, g: 0, b: 0 });
  const bands = [
    hues(p.primaryBlue, PRIMARY_COLUMNS),
    hues(p.primaryOrange.length ? p.primaryOrange : p.primaryBlue, PRIMARY_COLUMNS),
    hues(p.accent, ACCENT_COLUMNS),
  ];
  return (
    <div className="flex flex-col gap-[3px] overflow-hidden rounded-md">
      {bands.map((band, bi) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: fixed bands
        <div key={bi} className="flex h-2.5">
          {band.map((c, i) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: positional swatches
            <span key={i} className="flex-1" style={{ background: toHex(c) }} />
          ))}
        </div>
      ))}
    </div>
  );
}

export function PaletteLibrary({
  palettes,
  stock,
  selectedId,
  activeId,
  onSelect,
  onNew,
  onDuplicate,
  onDelete,
}: PaletteLibraryProps) {
  const { t } = useTranslation("palette");
  const items = [...(stock ? [stock] : []), ...(palettes ?? [])];

  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3">
      <button
        type="button"
        onClick={onNew}
        className="flex min-h-24 flex-col items-center justify-center gap-1.5 rounded-lg border border-line-strong border-dashed text-fg-muted transition-colors hover:border-[var(--color-accent)] hover:text-fg"
      >
        <Plus className="size-4" />
        <span className="text-[13px] font-medium">{t("actions.new")}</span>
        <span className="text-[11px] text-fg-subtle">{t("library.newHint")}</span>
      </button>
      {!palettes || !stock ? (
        <>
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
        </>
      ) : null}
      <AnimatePresence initial={false}>
        {items.map((p, i) => {
          const isStock = p.id === STOCK_ID;
          const active = activeId === p.id || (isStock && !activeId);
          return (
            <motion.div
              key={p.id}
              layout
              initial={{ opacity: 0, x: -12 }}
              animate={{ opacity: 1, x: 0, transition: { delay: i * 0.035 } }}
              exit={{ opacity: 0, x: -12 }}
            >
              <GlassCard
                selected={selectedId === p.id}
                className="cursor-pointer p-3.5"
                onClick={() => onSelect(p)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") onSelect(p);
                }}
              >
                <div className="mb-2.5 flex items-center justify-between gap-2">
                  <div className="min-w-0">
                    <p className="truncate text-sm font-semibold">{isStock ? t("library.stock") : p.name}</p>
                    <p className="text-[11px] text-fg-subtle">
                      {isStock ? t("library.stockHint") : formatRelative(p.updatedAt)}
                    </p>
                  </div>
                  <div className="flex shrink-0 items-center gap-1">
                    {active ? (
                      <Badge tone="success" dot pulse={!isStock}>
                        {t("library.inGame")}
                      </Badge>
                    ) : null}
                    <Tooltip content={t("actions.duplicate")}>
                      <Button
                        size="icon-sm"
                        variant="ghost"
                        aria-label={t("actions.duplicate")}
                        onClick={(e) => {
                          e.stopPropagation();
                          onDuplicate(p);
                        }}
                      >
                        <Copy />
                      </Button>
                    </Tooltip>
                    {!isStock ? (
                      <Tooltip content={t("actions.delete")}>
                        <Button
                          size="icon-sm"
                          variant="ghost"
                          aria-label={t("actions.delete")}
                          className="hover:text-danger"
                          onClick={(e) => {
                            e.stopPropagation();
                            onDelete(p);
                          }}
                        >
                          <Trash2 />
                        </Button>
                      </Tooltip>
                    ) : null}
                  </div>
                </div>
                <Strip p={p} />
              </GlassCard>
            </motion.div>
          );
        })}
      </AnimatePresence>
      {palettes && palettes.length === 0 ? (
        <p className="col-span-full px-1 text-xs text-fg-subtle">{t("library.empty")}</p>
      ) : null}
    </div>
  );
}
