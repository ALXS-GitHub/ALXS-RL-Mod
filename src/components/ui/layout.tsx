import type { LucideIcon } from "lucide-react";
import { motion } from "motion/react";
import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/cn";

/** Page wrapper: scroll container + consistent padding + enter animation. */
export function Page({ className, children }: { className?: string; children: ReactNode }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.2, ease: "easeOut" }}
      className={cn("mx-auto flex w-full max-w-[1320px] flex-col gap-5 px-8 pt-7 pb-12", className)}
    >
      {children}
    </motion.div>
  );
}

interface PageHeaderProps {
  icon?: LucideIcon;
  title: ReactNode;
  subtitle?: ReactNode;
  actions?: ReactNode;
  /** Section name shown as a breadcrumb above the title. */
  eyebrow?: ReactNode;
}

export function PageHeader({ icon: Icon, title, subtitle, actions, eyebrow }: PageHeaderProps) {
  return (
    <header className="flex flex-wrap items-end justify-between gap-4 border-line border-b pb-5">
      <div className="min-w-0">
        {eyebrow || Icon ? (
          <p className="mb-2 flex items-center gap-1.5 text-xs text-fg-subtle">
            {Icon ? <Icon className="size-3.5" /> : null}
            {eyebrow}
          </p>
        ) : null}
        <h1 className="truncate text-[22px] font-semibold leading-tight tracking-[-0.02em]">{title}</h1>
        {subtitle ? <p className="mt-1 max-w-2xl text-[13px] text-fg-muted">{subtitle}</p> : null}
      </div>
      {actions ? <div className="flex flex-wrap items-center gap-2">{actions}</div> : null}
    </header>
  );
}

interface SectionProps extends Omit<HTMLAttributes<HTMLElement>, "title"> {
  title?: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
}

export function Section({ title, description, actions, className, children, ...props }: SectionProps) {
  return (
    <section className={cn("flex flex-col gap-3", className)} {...props}>
      {title || actions ? (
        <div className="flex items-end justify-between gap-3">
          <div>
            {title ? <h2 className="text-sm font-semibold">{title}</h2> : null}
            {description ? <p className="text-xs text-fg-muted">{description}</p> : null}
          </div>
          {actions}
        </div>
      ) : null}
      {children}
    </section>
  );
}

/** Label + description on the left, control on the right. Settings rows. */
export function Field({
  label,
  description,
  children,
  className,
}: {
  label: ReactNode;
  description?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex items-center justify-between gap-6 py-3", className)}>
      <div className="min-w-0">
        <p className="text-[13px] font-medium">{label}</p>
        {description ? <p className="mt-0.5 text-xs text-fg-muted">{description}</p> : null}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

/** Big number tile. */
export function Stat({
  label,
  value,
  hint,
  tone = "default",
}: {
  label: ReactNode;
  value: ReactNode;
  hint?: ReactNode;
  tone?: "default" | "success" | "danger" | "accent";
}) {
  const toneClass = {
    default: "text-fg",
    success: "text-success",
    danger: "text-danger",
    accent: "text-[color-mix(in_oklab,var(--color-accent)_40%,white)]",
  }[tone];
  return (
    <div className="flex flex-col gap-0.5">
      <span className="text-xs text-fg-subtle">{label}</span>
      <span className={cn("text-[22px] font-semibold tracking-[-0.02em] tabular-nums", toneClass)}>
        {value}
      </span>
      {hint ? <span className="text-xs text-fg-muted">{hint}</span> : null}
    </div>
  );
}
