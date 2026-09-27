import { cva, type VariantProps } from "class-variance-authority";
import type { HTMLAttributes } from "react";
import { cn } from "@/lib/cn";

const badgeVariants = cva(
  "inline-flex h-5 items-center gap-1.5 rounded-[5px] px-1.5 text-[11px] font-medium whitespace-nowrap [&_svg]:size-3",
  {
    variants: {
      tone: {
        neutral: "bg-white/[0.06] text-fg-muted",
        accent:
          "bg-[color-mix(in_oklab,var(--color-accent)_18%,transparent)] text-[color-mix(in_oklab,var(--color-accent)_35%,white)]",
        success: "bg-success/12 text-success",
        warning: "bg-warning/12 text-warning",
        danger: "bg-danger/12 text-danger",
        info: "bg-info/12 text-info",
        blue: "bg-team-blue/15 text-[#9cc1ff]",
        orange: "bg-team-orange/15 text-[#ffbe8f]",
      },
    },
    defaultVariants: { tone: "neutral" },
  },
);

interface BadgeProps extends HTMLAttributes<HTMLSpanElement>, VariantProps<typeof badgeVariants> {
  /** Leading status dot (pulses when `pulse`). */
  dot?: boolean;
  pulse?: boolean;
}

export function Badge({ className, tone, dot, pulse, children, ...props }: BadgeProps) {
  return (
    <span className={cn(badgeVariants({ tone }), className)} {...props}>
      {dot ? (
        <span className={cn("size-1.5 rounded-full bg-current", pulse && "animate-pulse-soft")} aria-hidden />
      ) : null}
      {children}
    </span>
  );
}
