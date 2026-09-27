import { Columns3, Pipette, RotateCcw, Rows3 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Slider } from "@/components/ui/slider";
import type { Rgb } from "../api";
import { fromHex, toHex } from "../colors";

interface ColorEditorProps {
  index: number | null;
  color: Rgb | null;
  stockColor: Rgb | null;
  columns: number;
  onChange: (color: Rgb) => void;
  onFillRow: () => void;
  onFillColumn: () => void;
}

const CHANNELS = ["r", "g", "b"] as const;
const CHANNEL_TINT: Record<(typeof CHANNELS)[number], string> = {
  r: "#ff5d73",
  g: "#3ddc97",
  b: "#5cc8ff",
};

/** Side panel editing the selected slot. */
export function ColorEditor({
  index,
  color,
  stockColor,
  columns,
  onChange,
  onFillRow,
  onFillColumn,
}: ColorEditorProps) {
  const { t } = useTranslation("palette");
  const [hexDraft, setHexDraft] = useState(color ? toHex(color) : "");

  useEffect(() => {
    if (color) setHexDraft(toHex(color));
  }, [color]);

  if (index === null || !color) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 p-6 text-center">
        <Pipette className="size-6 text-fg-subtle" />
        <p className="text-sm text-fg-muted">{t("editor.selectHint")}</p>
      </div>
    );
  }

  const hex = toHex(color);
  const row = Math.floor(index / columns) + 1;
  const col = (index % columns) + 1;

  return (
    <div className="flex flex-col gap-4 p-4">
      <div className="flex items-center gap-4">
        <label
          className="group relative size-14 shrink-0 cursor-pointer overflow-hidden rounded-[8px] shadow-[inset_0_0_0_1px_rgb(255_255_255/0.12)]"
          style={{ background: hex }}
        >
          <input
            type="color"
            value={hex}
            onChange={(e) => {
              const c = fromHex(e.target.value);
              if (c) onChange(c);
            }}
            className="absolute inset-0 cursor-pointer opacity-0"
            aria-label={t("editor.pick")}
          />
          <Pipette className="absolute right-1.5 bottom-1.5 size-3.5 text-white/80 opacity-0 transition-opacity group-hover:opacity-100" />
        </label>
        <div className="min-w-0 space-y-1.5">
          <p className="text-xs text-fg-subtle">{t("editor.position", { row, col })}</p>
          <Input
            value={hexDraft}
            onChange={(e) => {
              setHexDraft(e.target.value);
              const c = fromHex(e.target.value);
              if (c) onChange(c);
            }}
            className="w-28 font-mono uppercase"
            aria-label={t("editor.hex")}
            spellCheck={false}
          />
        </div>
      </div>

      <div className="space-y-3">
        {CHANNELS.map((ch) => (
          <div key={ch} className="flex items-center gap-3">
            <span className="w-3 font-mono text-xs uppercase" style={{ color: CHANNEL_TINT[ch] }}>
              {ch}
            </span>
            <Slider
              value={[color[ch]]}
              min={0}
              max={255}
              step={1}
              onValueChange={([v]) => onChange({ ...color, [ch]: v ?? 0 })}
            />
            <span className="w-8 text-right font-mono text-xs tabular-nums text-fg-muted">{color[ch]}</span>
          </div>
        ))}
      </div>

      <div className="grid grid-cols-2 gap-2">
        <Button size="sm" variant="secondary" onClick={onFillRow}>
          <Rows3 />
          {t("editor.fillRow")}
        </Button>
        <Button size="sm" variant="secondary" onClick={onFillColumn}>
          <Columns3 />
          {t("editor.fillColumn")}
        </Button>
        {stockColor ? (
          <Button size="sm" variant="ghost" className="col-span-2" onClick={() => onChange(stockColor)}>
            <RotateCcw />
            {t("editor.resetSlot")}
            <span className="ml-1 size-3 rounded-sm" style={{ background: toHex(stockColor) }} />
          </Button>
        ) : null}
      </div>
    </div>
  );
}
