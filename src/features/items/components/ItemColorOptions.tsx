import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ColorPicker, type ColorPreset } from "@/components/ui/color-picker";
import { Segmented } from "@/components/ui/segmented";
import { cn } from "@/lib/cn";
import type { ColorBlend } from "../api";
import { useColorPreview } from "../queries";

/** One official paint (`id` = the game's PaintID). */
export interface PaintOption {
  id: number;
  label: string;
  hex: string;
}

export type ColorMode = "original" | "paint" | "custom";

/** What the colour options edit. `blend` only matters with a custom colour. */
export interface ItemColor {
  paint: number;
  color: string | null;
  blend: ColorBlend;
}

interface ItemColorOptionsProps extends ItemColor {
  onChange: (next: ItemColor) => void;
  /** Paints the shown item accepts (empty: not paintable). */
  paints: readonly PaintOption[];
  /** The shown item (for the colour preview). */
  itemId: number | null;
}

/** The value after `delay` ms without change (keeps the preview calm while dragging). */
function useSettled<T>(value: T, delay = 250): T {
  const [settled, setSettled] = useState(value);
  useEffect(() => {
    const id = window.setTimeout(() => setSettled(value), delay);
    return () => window.clearTimeout(id);
  }, [value, delay]);
  return settled;
}

/** Flat preview of the shown item in the custom colour (hidden when it has no colour texture). */
function ColorPreview({ itemId, color, blend }: { itemId: number | null; color: string; blend: ColorBlend }) {
  const { t } = useTranslation("items");
  const settled = useSettled(color);
  const preview = useColorPreview(itemId, settled, blend);
  if (!preview.data) return null;
  return (
    <figure className="flex flex-col gap-1.5">
      <img
        src={preview.data}
        alt={t("color.preview")}
        className={cn(
          "size-36 rounded-md border border-line bg-black object-contain transition-opacity",
          preview.isFetching && "opacity-60",
        )}
      />
      <figcaption className="text-[11px] text-fg-subtle">{t("color.preview")}</figcaption>
    </figure>
  );
}

/** Colours offered as quick picks in the custom mode. */
export const CUSTOM_PRESETS: readonly { key: string; hex: string }[] = [
  { key: "red", hex: "#ff0000" },
  { key: "orange", hex: "#ff6a00" },
  { key: "yellow", hex: "#ffd500" },
  { key: "green", hex: "#00e64d" },
  { key: "cyan", hex: "#00d5ff" },
  { key: "blue", hex: "#0055ff" },
  { key: "purple", hex: "#8c00ff" },
  { key: "pink", hex: "#ff0099" },
  { key: "white", hex: "#ffffff" },
];

export function colorMode(paint: number, color: string | null): ColorMode {
  if (color) return "custom";
  return paint > 0 ? "paint" : "original";
}

/**
 * Colour of the shown item: original, an official paint (paintable items)
 * or any custom colour (experimental recolour).
 */
export function ItemColorOptions({ paint, color, blend, onChange, paints, itemId }: ItemColorOptionsProps) {
  const { t } = useTranslation("items");
  const mode = colorMode(paint, color);
  const presets: ColorPreset[] = CUSTOM_PRESETS.map((p) => ({
    hex: p.hex,
    label: t(`color.presets.${p.key}`),
  }));
  const modes = [
    { value: "original" as const, label: t("color.modes.original") },
    ...(paints.length ? [{ value: "paint" as const, label: t("color.modes.paint") }] : []),
    { value: "custom" as const, label: t("color.modes.custom") },
  ];

  const blends = [
    { value: "exact" as const, label: t("color.blends.exact") },
    { value: "tint" as const, label: t("color.blends.tint") },
  ];

  const setMode = (next: ColorMode) => {
    if (next === "original") onChange({ paint: 0, color: null, blend });
    else if (next === "paint") onChange({ paint: paints[0]?.id ?? 0, color: null, blend });
    else onChange({ paint: 0, color: color ?? CUSTOM_PRESETS[0]?.hex ?? "#ff0000", blend });
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-3">
        <span className="text-xs text-fg-subtle">{t("color.label")}</span>
        <Segmented value={mode} onValueChange={setMode} size="sm" options={modes} />
      </div>
      {mode === "paint" ? (
        <div className="flex flex-wrap gap-1.5">
          {paints.map((p) => (
            <button
              key={p.id}
              type="button"
              aria-pressed={paint === p.id}
              onClick={() => onChange({ paint: p.id, color: null, blend })}
              className={cn(
                "flex h-7 items-center gap-1.5 rounded-md border border-line px-2 text-xs text-fg-muted transition-colors hover:text-fg",
                paint === p.id && "border-[var(--color-accent)] text-fg",
              )}
            >
              <span className="size-3.5 rounded-full border border-white/15" style={{ background: p.hex }} />
              {p.label}
            </button>
          ))}
        </div>
      ) : null}
      {mode === "custom" && color ? (
        <>
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-xs text-fg-subtle">{t("color.blendLabel")}</span>
            <Segmented
              value={blend}
              onValueChange={(next) => onChange({ paint: 0, color, blend: next })}
              size="sm"
              options={blends}
            />
          </div>
          <div className="flex flex-wrap items-start gap-4">
            <ColorPicker
              value={color}
              onChange={(hex) => onChange({ paint: 0, color: hex, blend })}
              presets={presets}
              size={148}
            />
            <ColorPreview itemId={itemId} color={color} blend={blend} />
          </div>
        </>
      ) : null}
      <p className="text-[11px] text-fg-subtle">
        {mode === "custom" ? t(`color.blendHints.${blend}`) : t(`color.hints.${mode}`)}
      </p>
    </div>
  );
}
