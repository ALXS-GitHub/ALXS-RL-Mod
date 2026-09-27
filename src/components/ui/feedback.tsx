import { AlertTriangle, type LucideIcon, RotateCw } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { toAppError } from "@/lib/errors";
import { Button } from "./button";

/** Loading placeholder with a glass shimmer. */
export function Skeleton({ className }: { className?: string }) {
  return <div className={cn("shimmer rounded-md", className)} aria-hidden />;
}

interface EmptyStateProps {
  icon: LucideIcon;
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({ icon: Icon, title, description, action, className }: EmptyStateProps) {
  return (
    <div className={cn("flex flex-col items-center justify-center gap-2 px-6 py-14 text-center", className)}>
      <div className="mb-1 grid size-9 place-items-center rounded-[8px] border border-line bg-white/[0.04]">
        <Icon className="size-4 text-fg-muted" />
      </div>
      <p className="text-sm font-medium">{title}</p>
      {description ? <p className="max-w-sm text-[13px] text-fg-muted">{description}</p> : null}
      {action ? <div className="mt-2">{action}</div> : null}
    </div>
  );
}

/** Renders any thrown error with a localised message and an optional retry. */
export function ErrorState({
  error,
  onRetry,
  className,
}: {
  error: unknown;
  onRetry?: () => void;
  className?: string;
}) {
  const { t } = useTranslation();
  const appError = toAppError(error);
  return (
    <div
      className={cn(
        "flex items-start gap-3 rounded-lg border border-danger/25 bg-danger/[0.06] p-3.5 text-[13px]",
        className,
      )}
      role="alert"
    >
      <AlertTriangle className="mt-0.5 size-4 shrink-0 text-danger" />
      <div className="min-w-0 flex-1 space-y-1">
        <p className="font-medium text-fg">
          {t(`errors.${appError.kind}`, { defaultValue: t("errors.title") })}
        </p>
        <p className="break-words text-xs text-fg-muted" data-selectable>
          {appError.message}
        </p>
      </div>
      {onRetry ? (
        <Button size="sm" variant="ghost" onClick={onRetry}>
          <RotateCw />
          {t("actions.retry")}
        </Button>
      ) : null}
    </div>
  );
}

export function Kbd({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <kbd
      className={cn(
        "inline-flex h-[18px] min-w-[18px] items-center justify-center rounded-[4px] border border-line bg-white/[0.04] px-1 font-sans text-[10px] font-medium text-fg-subtle",
        className,
      )}
    >
      {children}
    </kbd>
  );
}
