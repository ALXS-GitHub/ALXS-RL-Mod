import { ArrowRight, Check, Trash2, Wrench } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Select } from "@/components/ui/select";
import { notify } from "@/components/ui/toast";
import { formatRelative } from "@/lib/format";
import { type MapEntry, ORIGINAL_VARIANT } from "../api";
import { useDeleteRepair, useRepairMap, useRepairMappings, useRepairStatus, useUpdateMap } from "../queries";

/** `A.b.c.Leaf` → `Leaf (A)`: readable without the internal groups. */
function short(path: string): string {
  const parts = path.split(".");
  return parts.length > 1 ? `${parts[parts.length - 1]} (${parts[0]})` : path;
}

/**
 * Versions of a map: the original (always kept) and one repaired copy per
 * mapping version. Pick the default, delete a copy, or create one with any
 * mapping version (latest by default). The map keeps its name, thumbnail
 * and settings.
 */
export function RepairDialog({ map, onClose }: { map: MapEntry | null; onClose: () => void }) {
  const { t } = useTranslation("maps");
  const { data: mappings = [] } = useRepairMappings();
  const latest = mappings.find((m) => m.latest)?.version ?? null;
  const [picked, setPicked] = useState<number | null>(null);
  const version = picked ?? latest;
  const { data: statuses } = useRepairStatus(version);
  const repair = useRepairMap();
  const remove = useDeleteRepair();
  const update = useUpdateMap();
  const status = map ? statuses?.find((s) => s.mapId === map.id) : undefined;
  const repairs = map?.repairs ?? [];
  const already = repairs.some((r) => r.version === version);

  const close = () => {
    setPicked(null);
    onClose();
  };
  const setDefault = (v: number) => map && update.mutate({ id: map.id, patch: { preferredVariant: v } });

  const Row = ({ v, label, detail }: { v: number; label: string; detail: string }) => {
    const isDefault = (map?.preferredVariant ?? 0) === v;
    return (
      <li className="flex items-center justify-between gap-3 rounded-[7px] border border-line px-2.5 py-2">
        <span className="min-w-0">
          <span className="flex items-center gap-1.5 text-[13px] font-medium">
            {label}
            {isDefault ? <Badge tone="accent">{t("repair.default")}</Badge> : null}
          </span>
          <span className="block truncate text-xs text-fg-subtle">{detail}</span>
        </span>
        <span className="flex shrink-0 items-center gap-1">
          {isDefault ? null : (
            <Button size="sm" variant="ghost" loading={update.isPending} onClick={() => setDefault(v)}>
              <Check />
              {t("repair.makeDefault")}
            </Button>
          )}
          {v === ORIGINAL_VARIANT ? null : (
            <Button
              size="icon-sm"
              variant="ghost"
              className="hover:text-danger"
              aria-label={t("repair.deleteCopy")}
              title={t("repair.deleteCopy")}
              loading={remove.isPending && remove.variables?.version === v}
              onClick={() =>
                map &&
                remove.mutate(
                  { id: map.id, version: v },
                  { onSuccess: () => notify.success(t("repair.copyDeleted", { version: v })) },
                )
              }
            >
              <Trash2 />
            </Button>
          )}
        </span>
      </li>
    );
  };

  return (
    <Dialog open={map !== null} onOpenChange={(o) => !o && close()}>
      <DialogContent
        size="md"
        title={t("repair.title", { name: map?.name ?? "" })}
        description={t("repair.description")}
      >
        <div className="flex flex-col gap-5">
          <section className="flex flex-col gap-2">
            <h3 className="text-xs font-semibold text-fg-muted">{t("repair.versions")}</h3>
            <ul className="flex flex-col gap-1.5">
              <Row v={ORIGINAL_VARIANT} label={t("repair.original")} detail={t("repair.originalDetail")} />
              {repairs.map((r) => (
                <Row
                  key={r.version}
                  v={r.version}
                  label={t("repair.repairedLabel", { version: r.version })}
                  detail={t("repair.repairedDetail", {
                    count: r.fixes.length,
                    when: formatRelative(r.repairedAt),
                  })}
                />
              ))}
            </ul>
            <p className="text-xs text-fg-subtle">{t("repair.defaultNote")}</p>
          </section>

          <section className="flex flex-col gap-2 border-line border-t pt-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <h3 className="text-xs font-semibold text-fg-muted">{t("repair.create")}</h3>
              <Select
                className="w-auto min-w-64"
                value={version !== null ? String(version) : undefined}
                onValueChange={(v) => setPicked(Number(v))}
                options={mappings
                  .slice()
                  .reverse()
                  .map((m) => ({
                    value: String(m.version),
                    label: `v${m.version}${m.latest ? ` · ${t("repair.latest")}` : ""} — ${m.label}`,
                  }))}
              />
            </div>
            {!status ? (
              <p className="text-xs text-fg-subtle">{t("repair.checking")}</p>
            ) : !status.supported ? (
              <p className="text-xs text-warning">{t("repair.unsupported")}</p>
            ) : status.fixes.length === 0 ? (
              <p className="text-xs text-fg-muted">{t("repair.nothing", { version })}</p>
            ) : (
              <>
                <p className="text-xs text-fg-muted">
                  {t("repair.willFix", { count: status.fixes.length, version })}
                </p>
                <ul className="flex max-h-48 flex-col gap-1 overflow-y-auto rounded-[8px] border border-line p-2 text-xs">
                  {status.fixes.map((f) => (
                    <li key={f.from} className="flex items-center gap-1.5" title={`${f.from} → ${f.to}`}>
                      <span className="truncate text-fg-muted">{short(f.from)}</span>
                      <ArrowRight className="size-3 shrink-0 text-fg-subtle" />
                      <span className="truncate text-fg">{short(f.to)}</span>
                    </li>
                  ))}
                </ul>
              </>
            )}
            <div className="flex justify-end">
              <Button
                variant="primary"
                loading={repair.isPending}
                disabled={!map || version === null || !status?.supported || !status.fixes.length}
                onClick={() =>
                  map &&
                  version !== null &&
                  repair.mutate(
                    { id: map.id, version },
                    { onSuccess: () => notify.success(t("repair.done", { name: map.name, version })) },
                  )
                }
              >
                <Wrench />
                {already ? t("repair.redo", { version }) : t("repair.apply", { version })}
              </Button>
            </div>
            <p className="text-xs text-fg-subtle">{t("repair.keepNote")}</p>
          </section>
        </div>
      </DialogContent>
    </Dialog>
  );
}
