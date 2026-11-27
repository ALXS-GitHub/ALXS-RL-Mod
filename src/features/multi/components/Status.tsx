import { Check, CircleAlert } from "lucide-react";
import type { ReactNode } from "react";

/** One status line: green check when fine, amber otherwise. */
export function Status({ ok, children }: { ok: boolean; children: ReactNode }) {
  return (
    <p className="flex items-start gap-1.5 text-xs text-fg-muted">
      {ok ? (
        <Check className="mt-0.5 size-3.5 shrink-0 text-success" />
      ) : (
        <CircleAlert className="mt-0.5 size-3.5 shrink-0 text-warning" />
      )}
      <span>{children}</span>
    </p>
  );
}
