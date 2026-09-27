import { ArrowRight } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { CatalogItem, SwapRequest } from "@/features/items/api";
import { ItemThumb } from "@/features/items/components/ItemThumb";
import { itemLabel, SLOT_ICONS } from "@/features/items/constants";

/** Compact owned → wanted rows (preview of a preset's swaps). */
export function SwapList({ swaps, byId }: { swaps: SwapRequest[]; byId: Map<number, CatalogItem> }) {
  const { t } = useTranslation("items");
  return (
    <ul className="flex flex-col gap-1.5">
      {swaps.map((s) => {
        const owned = byId.get(s.ownedId);
        const wanted = byId.get(s.wantedId);
        const Icon = SLOT_ICONS[s.slot];
        return (
          <li
            key={`${s.slot}-${s.ownedId}-${s.wantedId}`}
            className="flex items-center gap-3 rounded-[8px] border border-line bg-white/[0.02] p-2"
          >
            <ItemThumb item={wanted} className="size-11 shrink-0" />
            <div className="min-w-0 flex-1">
              <p className="flex items-center gap-1.5 text-[11px] text-fg-subtle">
                <Icon className="size-3" />
                {t(`slots.${s.slot}`)}
              </p>
              <p className="truncate text-[13px] font-medium">
                {wanted ? itemLabel(wanted) : `#${s.wantedId}`}
              </p>
            </div>
            <ArrowRight className="size-3.5 shrink-0 text-fg-subtle" />
            <p className="w-40 shrink-0 truncate text-right text-xs text-fg-muted">
              {owned ? itemLabel(owned) : `#${s.ownedId}`}
            </p>
          </li>
        );
      })}
    </ul>
  );
}
