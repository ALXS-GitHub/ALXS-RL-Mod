import { Dices, Save } from "lucide-react";
import { motion } from "motion/react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogTrigger } from "@/components/ui/dialog";
import { SLOTS, type Slot } from "@/features/items/api";
import { SLOT_ICONS } from "@/features/items/constants";
import { useCatalog } from "@/features/items/queries";
import { cn } from "@/lib/cn";
import type { Preset } from "../api";
import { useRandomPreset, useSavePreset } from "../queries";
import { SwapList } from "./SwapList";

const DEFAULT_SLOTS: Slot[] = ["body", "decal", "wheels", "boost", "goalExplosion", "trail"];

export function RandomDialog() {
  const { t } = useTranslation("presets");
  const { t: tItems } = useTranslation("items");
  const [open, setOpen] = useState(false);
  const [slots, setSlots] = useState<Slot[]>(DEFAULT_SLOTS);
  const [preview, setPreview] = useState<Preset | null>(null);
  const random = useRandomPreset();
  const save = useSavePreset();
  const { data: catalog } = useCatalog();
  const byId = useMemo(() => new Map(catalog?.items.map((i) => [i.id, i])), [catalog]);

  const toggle = (slot: Slot) =>
    setSlots((prev) => (prev.includes(slot) ? prev.filter((s) => s !== slot) : [...prev, slot]));

  const roll = () =>
    random.mutate(slots, { onSuccess: (p) => setPreview({ ...p, name: t("random.defaultName") }) });

  const missing = preview ? slots.filter((s) => !preview.swaps.some((w) => w.slot === s)) : [];

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        setOpen(o);
        if (!o) setPreview(null);
      }}
    >
      <DialogTrigger asChild>
        <Button variant="secondary">
          <Dices />
          {t("random.open")}
        </Button>
      </DialogTrigger>
      <DialogContent
        title={t("random.title")}
        description={t("random.description")}
        size="lg"
        footer={
          <>
            <Button
              variant="secondary"
              disabled={slots.length === 0}
              loading={random.isPending}
              onClick={roll}
            >
              <Dices />
              {preview ? t("random.reroll") : t("random.roll")}
            </Button>
            <Button
              variant="primary"
              disabled={!preview}
              loading={save.isPending}
              onClick={() => preview && save.mutate(preview, { onSuccess: () => setOpen(false) })}
            >
              <Save />
              {t("random.save")}
            </Button>
          </>
        }
      >
        <div className="flex flex-col gap-5">
          <div className="flex flex-wrap gap-2">
            {SLOTS.map((slot) => {
              const Icon = SLOT_ICONS[slot];
              const on = slots.includes(slot);
              return (
                <button
                  key={slot}
                  type="button"
                  aria-pressed={on}
                  onClick={() => toggle(slot)}
                  className={cn(
                    "inline-flex h-8 items-center gap-2 rounded-[7px] border px-3 text-[13px] transition-colors",
                    on
                      ? "border-[var(--color-accent)] bg-[color-mix(in_oklab,var(--color-accent)_10%,transparent)] text-fg"
                      : "border-line text-fg-muted hover:border-line-strong hover:text-fg",
                  )}
                >
                  <Icon className="size-3.5" />
                  {tItems(`slots.${slot}`)}
                </button>
              );
            })}
          </div>

          {preview ? (
            <motion.div
              key={preview.swaps.map((s) => s.wantedId).join("-")}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
            >
              <SwapList swaps={preview.swaps} byId={byId} />
              {missing.length > 0 ? (
                <p className="mt-3 text-xs text-fg-subtle">
                  {t("random.skipped", { slots: missing.map((s) => tItems(`slots.${s}`)).join(", ") })}
                </p>
              ) : null}
            </motion.div>
          ) : (
            <p className="text-sm text-fg-muted">{t("random.hint")}</p>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
