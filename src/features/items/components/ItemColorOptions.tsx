import { useTranslation } from "react-i18next";
import { ColorPicker, type ColorPreset } from "@/components/ui/color-picker";
import { Segmented } from "@/components/ui/segmented";
import { cn } from "@/lib/cn";

/** One official paint (`id` = the game's PaintID). */
export interface PaintOption {
  id: number;
  label: string;
  hex: string;
}

export type ColorMode = "original" | "paint" | "custom";

interface ItemColorOptionsProps {
  paint: number;
  color: string | null;
  onChange: (next: { paint: number; color: string | null }) => void;
  /** Paints the shown item accepts (empty: not paintable). */
  paints: readonly PaintOption[];
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
export function ItemColorOptions({ paint, color, onChange, paints }: ItemColorOptionsProps) {
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

  const setMode = (next: ColorMode) => {
    if (next === "original") onChange({ paint: 0, color: null });
    else if (next === "paint") onChange({ paint: paints[0]?.id ?? 0, color: null });
    else onChange({ paint: 0, color: color ?? CUSTOM_PRESETS[0]?.hex ?? "#ff0000" });
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
              onClick={() => onChange({ paint: p.id, color: null })}
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
        <ColorPicker
          value={color}
          onChange={(hex) => onChange({ paint: 0, color: hex })}
          presets={presets}
          size={148}
        />
      ) : null}
      <p className="text-[11px] text-fg-subtle">{t(`color.hints.${mode}`)}</p>
    </div>
  );
}
