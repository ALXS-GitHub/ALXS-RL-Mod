import { Bookmark, Save, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { notify } from "@/components/ui/toast";
import { targetLabel } from "@/features/maps/api";
import { cn } from "@/lib/cn";
import type { HostPreset } from "../../api";
import { useDeleteHostPreset, useHostPresets, useSaveHostPreset } from "../../queries";
import type { HostForm } from "./useHostForm";

/** Saved host setups: click one to load it, save the current form as one. */
export function PresetBar({
  form,
  target,
  mapIds,
  onApply,
  onSaved,
}: {
  form: HostForm;
  /** Arena actually used (form.target resolved). */
  target: string;
  /** Maps still in the library (presets of deleted maps are flagged). */
  mapIds: Set<string>;
  onApply: (preset: HostPreset) => void;
  onSaved: (preset: HostPreset) => void;
}) {
  const { t } = useTranslation("multi");
  const { data: presets = [] } = useHostPresets();
  const remove = useDeleteHostPreset();
  const [saving, setSaving] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const current = presets.find((p) => p.id === form.presetId) ?? null;

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <h3 className="flex items-center gap-1.5 text-xs font-semibold text-fg-muted">
          <Bookmark className="size-3.5" />
          {t("host.presets.title")}
        </h3>
        <Button size="sm" variant="ghost" disabled={!form.mapId} onClick={() => setSaving(true)}>
          <Save />
          {t("host.presets.save")}
        </Button>
      </div>
      {presets.length ? (
        <div className="flex flex-wrap gap-2">
          {presets.map((p) => {
            const missing = !mapIds.has(p.mapId);
            const active = p.id === form.presetId;
            return (
              <div
                key={p.id}
                className={cn(
                  "group flex items-center rounded-[7px] border transition-colors",
                  active
                    ? "border-[var(--color-accent)] bg-white/[0.04]"
                    : "border-line hover:border-line-strong",
                )}
              >
                <button
                  type="button"
                  disabled={missing}
                  title={missing ? t("host.presets.mapMissing", { name: p.mapName }) : undefined}
                  onClick={() => onApply(p)}
                  className="flex min-w-0 flex-col items-start px-2.5 py-1.5 text-left disabled:opacity-50"
                >
                  <span className="max-w-48 truncate text-xs font-medium text-fg">{p.name}</span>
                  <span className="max-w-48 truncate text-[11px] text-fg-subtle">
                    {p.mapName} · {targetLabel(p.target)}
                  </span>
                </button>
                <button
                  type="button"
                  aria-label={t("host.presets.delete")}
                  title={confirmDelete === p.id ? t("host.presets.confirmDelete") : t("host.presets.delete")}
                  onClick={() => {
                    if (confirmDelete !== p.id) return setConfirmDelete(p.id);
                    setConfirmDelete(null);
                    remove.mutate(p.id);
                  }}
                  onBlur={() => setConfirmDelete(null)}
                  className={cn(
                    "grid h-full place-items-center px-2 transition-opacity",
                    confirmDelete === p.id
                      ? "text-danger opacity-100"
                      : "text-fg-subtle opacity-0 group-hover:opacity-100 focus-visible:opacity-100",
                  )}
                >
                  <Trash2 className="size-3.5" />
                </button>
              </div>
            );
          })}
        </div>
      ) : (
        <p className="text-xs text-fg-subtle">{t("host.presets.empty")}</p>
      )}
      {saving ? (
        <SavePresetDialog
          form={form}
          target={target}
          current={current}
          onClose={() => setSaving(false)}
          onSaved={onSaved}
        />
      ) : null}
    </div>
  );
}

function SavePresetDialog({
  form,
  target,
  current,
  onClose,
  onSaved,
}: {
  form: HostForm;
  target: string;
  current: HostPreset | null;
  onClose: () => void;
  onSaved: (preset: HostPreset) => void;
}) {
  const { t } = useTranslation("multi");
  const save = useSaveHostPreset();
  const [name, setName] = useState(current?.name ?? "");
  const submit = (overwrite: boolean) => {
    if (!form.mapId || !name.trim()) return;
    save.mutate(
      {
        id: overwrite ? (current?.id ?? null) : null,
        name,
        mapId: form.mapId,
        target,
        variant: form.variant,
        options: form.options,
      },
      {
        onSuccess: (p) => {
          notify.success(t("host.presets.saved", { name: p.name }));
          onSaved(p);
          onClose();
        },
      },
    );
  };

  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent
        size="sm"
        title={t("host.presets.saveTitle")}
        description={t("host.presets.saveDescription")}
        footer={
          <>
            {current ? (
              <Button
                variant="secondary"
                loading={save.isPending}
                disabled={!name.trim()}
                onClick={() => submit(true)}
              >
                {t("host.presets.update", { name: current.name })}
              </Button>
            ) : null}
            <Button
              variant="primary"
              loading={save.isPending}
              disabled={!name.trim()}
              onClick={() => submit(false)}
            >
              {current ? t("host.presets.saveNew") : t("host.presets.save")}
            </Button>
          </>
        }
      >
        <Input
          autoFocus
          value={name}
          maxLength={60}
          placeholder={t("host.presets.namePlaceholder")}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") submit(false);
          }}
        />
      </DialogContent>
    </Dialog>
  );
}
