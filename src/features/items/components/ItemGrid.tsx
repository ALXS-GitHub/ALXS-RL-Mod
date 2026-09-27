import { useVirtualizer } from "@tanstack/react-virtual";
import { Layers, Lock } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { GlassCard } from "@/components/ui/glass";
import { Tooltip } from "@/components/ui/tooltip";
import type { CatalogItem } from "../api";
import { itemLabel } from "../constants";
import { ItemThumb } from "./ItemThumb";

const MIN_TILE = 148;
const GAP = 12;
const ROW_HEIGHT = 184;

interface ItemGridProps {
  items: readonly CatalogItem[];
  selectedId: number | null;
  /** Items that must not be picked (e.g. the owned item in the wanted step). */
  disabledIds?: ReadonlySet<number>;
  /** Ids currently swapped (shown with a badge). */
  swappedIds?: ReadonlySet<number>;
  /** Dim locked items (they cannot be a swap source in this step). */
  dimLocked?: boolean;
  onSelect: (item: CatalogItem) => void;
}

/**
 * Virtualised responsive grid: only visible rows are mounted, so thousands
 * of decals stay smooth and thumbnails load lazily as rows scroll in.
 */
export function ItemGrid({ items, selectedId, disabledIds, swappedIds, dimLocked, onSelect }: ItemGridProps) {
  const { t } = useTranslation("items");
  const scrollRef = useRef<HTMLDivElement>(null);
  const [columns, setColumns] = useState(5);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const observer = new ResizeObserver(([entry]) => {
      const width = entry?.contentRect.width ?? 0;
      setColumns(Math.max(2, Math.floor((width + GAP) / (MIN_TILE + GAP))));
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const rowCount = Math.ceil(items.length / columns);
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT + GAP,
    overscan: 3,
  });

  // Back to top when the list changes (slot, search, step).
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset only when the dataset identity changes
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 });
  }, [items]);

  return (
    <div ref={scrollRef} className="h-full min-h-0 overflow-y-auto pr-1">
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((row) => {
          const start = row.index * columns;
          const rowItems = items.slice(start, start + columns);
          return (
            <div
              key={row.key}
              className="absolute inset-x-0 grid"
              style={{
                transform: `translateY(${row.start}px)`,
                gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
                gap: GAP,
                height: ROW_HEIGHT,
              }}
            >
              {rowItems.map((item) => {
                const disabled = disabledIds?.has(item.id) ?? false;
                const selected = item.id === selectedId;
                return (
                  <GlassCard
                    key={item.id}
                    role="button"
                    tabIndex={disabled ? -1 : 0}
                    aria-pressed={selected}
                    aria-disabled={disabled}
                    selected={selected}
                    onClick={() => !disabled && onSelect(item)}
                    onKeyDown={(e) => {
                      if (!disabled && (e.key === "Enter" || e.key === " ")) {
                        e.preventDefault();
                        onSelect(item);
                      }
                    }}
                    className={
                      disabled
                        ? "flex cursor-not-allowed flex-col gap-2 p-2.5 opacity-35"
                        : dimLocked && item.locked
                          ? "flex cursor-help flex-col gap-2 p-2.5 opacity-55"
                          : "flex cursor-pointer flex-col gap-2 p-2.5"
                    }
                  >
                    <ItemThumb item={item} className="aspect-[4/3] w-full" />
                    <div className="flex min-h-0 flex-1 flex-col justify-between gap-1">
                      <p className="line-clamp-2 text-[12.5px] font-medium leading-snug">{itemLabel(item)}</p>
                      <div className="flex items-center gap-1">
                        {swappedIds?.has(item.id) ? (
                          <Badge tone="accent" className="px-1.5 py-0 text-[10px]">
                            {t("grid.swapped")}
                          </Badge>
                        ) : null}
                        {item.locked ? (
                          <Tooltip content={t("grid.lockedHint")}>
                            <Badge tone="warning" className="h-[18px] px-1 text-[10px]">
                              <Lock />
                              {t("grid.locked")}
                            </Badge>
                          </Tooltip>
                        ) : null}
                        {item.sharedPackage ? (
                          <Tooltip content={t("grid.sharedHint")}>
                            <span className="inline-flex text-fg-subtle">
                              <Layers className="size-3" />
                            </span>
                          </Tooltip>
                        ) : null}
                      </div>
                    </div>
                  </GlassCard>
                );
              })}
            </div>
          );
        })}
      </div>
    </div>
  );
}
