import { ArrowRight, RotateCcw, Wrench } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Select } from "@/components/ui/select";
import { notify } from "@/components/ui/toast";
import type { MapEntry } from "../api";
import { useRepairMap, useRepairMappings, useRepairStatus, useUndoRepair } from "../queries";

/** `A.b.c.Leaf` → `Leaf (A)`: readable without the internal groups. */
function short(path: string): string {
  const parts = path.split(".");
  return parts.length > 1 ? `${parts[parts.length - 1]} (${parts[0]})` : path;
}

/**
 * Repair a map with a chosen mapping version (latest by default). The
 * original package is kept; the map keeps its name, thumbnail and settings.
 */
export function RepairDialog({ map, onClose }: { map: MapEntry | null; onClose: () => void }) {
  const { t } = useTranslation("maps");
  const { data: mappings = [] } = useRepairMappings();
  const latest = mappings.find((m) => m.latest)?.version ?? null;
  const [picked, setPicked] = useState<number | null>(null);
  const version = picked ?? latest;
  const { data: statuses } = useRepairStatus(version);
  const repair = useRepairMap();
  const undo = useUndoRepair();
  const status = map ? statuses?.find((s) => s.mapId === map.id) : undefined;
  const current = map?.repair ?? null;
  const sameAsCurrent = current?.version === version;

  const close = () => {
    setPicked(null);
    onClose();
  };

  return (
    <Dialog open={map !== null} onOpenChange={(o) => !o && close()}>
      <DialogContent
        size="md"
        title={t("repair.title", { name: map?.name ?? "" })}
        description={t("repair.description")}
        footer={
          <>
            {current ? (
              <Button
                variant="ghost"
                loading={undo.isPending}
                onClick={() =>
                  map &&
                  undo.mutate(map.id, {
                    onSuccess: () => {
                      notify.success(t("repair.undone", { name: map.name }));
                      close();
                    },
                  })
                }
              >
                <RotateCcw />
                {t("repair.undo")}
              </Button>
            ) : null}
            <Button
              variant="primary"
              loading={repair.isPending}
              disabled={
                !map || version === null || !status?.supported || !status.fixes.length || sameAsCurrent
              }
              onClick={() =>
                map &&
                version !== null &&
                repair.mutate(
                  { id: map.id, version },
                  {
                    onSuccess: () => {
                      notify.success(t("repair.done", { name: map.name, version }));
                      close();
                    },
                  },
                )
              }
            >
              <Wrench />
              {current && !sameAsCurrent ? t("repair.update", { version }) : t("repair.apply", { version })}
            </Button>
          </>
        }
      >
        <div className="flex flex-col gap-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <span className="text-[13px]">{t("repair.mapping")}</span>
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
          {current ? (
            <p className="text-xs text-fg-muted">
              <Badge tone="success">{t("repair.repairedWith", { version: current.version })}</Badge>{" "}
              {t("repair.currentFixes", { count: current.fixes.length })}
            </p>
          ) : null}
          {!status ? (
            <p className="text-xs text-fg-subtle">{t("repair.checking")}</p>
          ) : !status.supported ? (
            <p className="text-xs text-warning">{t("repair.unsupported")}</p>
          ) : status.fixes.length === 0 ? (
            <p className="text-xs text-fg-muted">{t("repair.nothing", { version })}</p>
          ) : (
            <div className="flex flex-col gap-1.5">
              <p className="text-xs text-fg-muted">
                {t("repair.willFix", { count: status.fixes.length, version })}
              </p>
              <ul className="flex max-h-64 flex-col gap-1 overflow-y-auto rounded-[8px] border border-line p-2 text-xs">
                {status.fixes.map((f) => (
                  <li key={f.from} className="flex items-center gap-1.5" title={`${f.from} → ${f.to}`}>
                    <span className="truncate text-fg-muted">{short(f.from)}</span>
                    <ArrowRight className="size-3 shrink-0 text-fg-subtle" />
                    <span className="truncate text-fg">{short(f.to)}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}
          <p className="text-xs text-fg-subtle">{t("repair.keepNote")}</p>
        </div>
      </DialogContent>
    </Dialog>
  );
}
