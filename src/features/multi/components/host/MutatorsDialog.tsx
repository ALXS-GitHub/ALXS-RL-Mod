import { RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogClose, DialogContent } from "@/components/ui/dialog";
import { Select } from "@/components/ui/select";
import { cn } from "@/lib/cn";
import type { RpCatalog } from "../../api";
import { groupOf, MUTATOR_GROUPS } from "./mutatorGroups";

/** Select needs a non-empty value for "the game's default". */
const DEFAULT = "__default";

/** Every mutator, by category (match, ball, boost…), like a private match. */
export function MutatorsDialog({
  open,
  onOpenChange,
  catalog,
  value,
  onChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  catalog: RpCatalog;
  value: Record<string, string>;
  onChange: (next: Record<string, string>) => void;
}) {
  const { t } = useTranslation("multi");
  const changed = Object.keys(value).length;
  const set = (category: string, v: string | null) => {
    const next = { ...value };
    if (v) next[category] = v;
    else delete next[category];
    onChange(next);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        size="lg"
        title={t("host.mutators.title")}
        description={t("host.mutators.description")}
        footer={
          <>
            <Button variant="ghost" disabled={!changed} onClick={() => onChange({})}>
              <RotateCcw />
              {t("host.mutators.reset")}
            </Button>
            <DialogClose asChild>
              <Button variant="primary">{t("host.mutators.done")}</Button>
            </DialogClose>
          </>
        }
      >
        <div className="grid gap-x-6 gap-y-5 md:grid-cols-2">
          {MUTATOR_GROUPS.map((group) => {
            const items = catalog.mutators.filter((m) => groupOf(m.category) === group.key);
            if (!items.length) return null;
            return (
              <section key={group.key} className="flex flex-col gap-1.5">
                <h3 className="border-line border-b pb-1.5 text-xs font-semibold text-fg-muted">
                  {t(`host.mutators.groups.${group.key}`)}
                </h3>
                {items.map((m) => {
                  const current = value[m.category];
                  return (
                    <div
                      key={m.category}
                      className="grid grid-cols-[minmax(0,1fr)_minmax(0,1.1fr)] items-center gap-3"
                    >
                      <span
                        className={cn("truncate text-xs", current ? "font-medium text-fg" : "text-fg-muted")}
                      >
                        {m.label}
                      </span>
                      <Select
                        value={current ?? DEFAULT}
                        onValueChange={(v) => set(m.category, v === DEFAULT ? null : v)}
                        options={m.values.map((v) => ({
                          value: v.internal || DEFAULT,
                          label: v.display || t("host.mutators.default"),
                        }))}
                      />
                    </div>
                  );
                })}
              </section>
            );
          })}
        </div>
      </DialogContent>
    </Dialog>
  );
}
