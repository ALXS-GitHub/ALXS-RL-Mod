import { motion } from "motion/react";
import { type ReactNode, useId } from "react";
import { cn } from "@/lib/cn";

interface SegmentedProps<T extends string> {
  value: T;
  onValueChange: (value: T) => void;
  options: readonly { value: T; label: ReactNode; icon?: ReactNode }[];
  size?: "sm" | "md";
  className?: string;
  "aria-label"?: string;
}

/** Compact exclusive choice with a sliding flat pill. */
export function Segmented<T extends string>({
  value,
  onValueChange,
  options,
  size = "md",
  className,
  ...aria
}: SegmentedProps<T>) {
  const id = useId();
  return (
    <div
      role="radiogroup"
      className={cn("inline-flex rounded-[8px] border border-line bg-white/[0.03] p-0.5", className)}
      {...aria}
    >
      {options.map((o) => {
        const active = o.value === value;
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => onValueChange(o.value)}
            className={cn(
              "relative inline-flex items-center gap-1.5 rounded-[6px] font-medium transition-colors [&_svg]:size-3.5",
              size === "sm" ? "h-6 px-2.5 text-xs" : "h-7 px-3 text-[13px]",
              active ? "text-fg" : "text-fg-muted hover:text-fg",
            )}
          >
            {active ? (
              <motion.span
                layoutId={id}
                className="absolute inset-0 -z-10 rounded-[6px] bg-white/[0.09]"
                transition={{ type: "spring", stiffness: 500, damping: 40 }}
              />
            ) : null}
            {o.icon}
            {o.label}
          </button>
        );
      })}
    </div>
  );
}
