import type { HTMLAttributes } from "react";
import { cn } from "@/lib/cn";

/** Static translucent surface. Use for layout containers. */
export function GlassPanel({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("glass rounded-lg", className)} {...props} />;
}

interface GlassCardProps extends HTMLAttributes<HTMLDivElement> {
  /** Hairline brightens on hover. Default true. */
  interactive?: boolean;
  /** Selected / active state: crisp accent ring and a faint accent wash. */
  selected?: boolean;
}

/** The surface every feature page is made of. Flat, 1px hairline, no glow. */
export function GlassCard({ className, interactive = true, selected = false, ...props }: GlassCardProps) {
  return (
    <div
      data-selected={selected || undefined}
      className={cn(
        "glass rounded-lg",
        interactive && "spotlight",
        selected &&
          "border-[color-mix(in_oklab,var(--color-accent)_75%,transparent)] bg-[color-mix(in_oklab,var(--color-accent)_7%,var(--color-surface))] hover:border-[var(--color-accent)]",
        className,
      )}
      {...props}
    />
  );
}
