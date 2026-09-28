import { Columns3, Pipette, RotateCcw, Rows3 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { ColorPicker } from "@/components/ui/color-picker";
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

  if (index === null || !color) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 p-6 text-center">
        <Pipette className="size-6 text-fg-subtle" />
        <p className="text-sm text-fg-muted">{t("editor.selectHint")}</p>
      </div>
    );
  }

  const row = Math.floor(index / columns) + 1;
  const col = (index % columns) + 1;

  return (
    <div className="flex flex-col gap-4 p-4">
      <p className="text-xs text-fg-subtle">{t("editor.position", { row, col })}</p>
      <ColorPicker
        value={toHex(color)}
        size={148}
        onChange={(hex) => {
          const c = fromHex(hex);
          if (c) onChange(c);
        }}
      />

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
