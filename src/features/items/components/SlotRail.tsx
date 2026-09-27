import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { formatNumber } from "@/lib/format";
import { SLOTS, type Slot } from "../api";
import { SLOT_ICONS } from "../constants";

interface SlotRailProps {
  value: Slot;
  onChange: (slot: Slot) => void;
  counts: Partial<Record<Slot, number>>;
  /** Slots with an active swap get a dot. */
  activeSlots: ReadonlySet<Slot>;
}

export function SlotRail({ value, onChange, counts, activeSlots }: SlotRailProps) {
  const { t } = useTranslation("items");
  return (
    <nav aria-label={t("slots.title")} className="glass flex flex-col gap-px rounded-lg p-1.5">
      <p className="px-2.5 pt-1 pb-1.5 text-xs text-fg-subtle">{t("slots.title")}</p>
      {SLOTS.map((slot) => {
        const Icon = SLOT_ICONS[slot];
        const active = slot === value;
        const count = counts[slot] ?? 0;
        return (
          <button
            key={slot}
            type="button"
            disabled={count === 0}
            onClick={() => onChange(slot)}
            className={cn(
              "group relative flex h-8 items-center gap-2.5 rounded-[7px] px-2.5 text-left text-[13px] font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-35",
              active ? "text-fg" : "text-fg-muted hover:bg-white/[0.04] hover:text-fg",
            )}
          >
            {active ? (
              <motion.span
                layoutId="items-slot-active"
                transition={{ type: "spring", stiffness: 500, damping: 42 }}
                className="absolute inset-0 -z-10 rounded-[7px] bg-white/[0.08]"
              />
            ) : null}
            <Icon
              className={cn("size-4 shrink-0", active ? "text-[var(--color-accent)]" : "text-fg-subtle")}
            />
            <span className="flex-1 truncate">{t(`slots.${slot}`)}</span>
            {activeSlots.has(slot) ? (
              <span className="size-1.5 rounded-full bg-[var(--color-accent)]" />
            ) : null}
            <span className="text-[11px] tabular-nums text-fg-subtle">{formatNumber(count)}</span>
          </button>
        );
      })}
    </nav>
  );
}
