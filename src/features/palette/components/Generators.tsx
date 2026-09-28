import { Blend, Dices, Rainbow, RotateCcw, SunMoon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { ColorSwatchButton } from "@/components/ui/color-picker";
import { Slider } from "@/components/ui/slider";
import { Tooltip } from "@/components/ui/tooltip";
import type { Rgb } from "../api";
import { fromHex, gradientGrid, harmonyGrid, hueShift, rainbowGrid } from "../colors";

interface GeneratorsProps {
  columns: number;
  rows: number;
  current: readonly Rgb[];
  stock: readonly Rgb[];
  onReplace: (colors: Rgb[]) => void;
}

/** One-click whole-grid generators for the visible team. */
export function Generators({ columns, rows, current, stock, onReplace }: GeneratorsProps) {
  const { t } = useTranslation("palette");
  const [from, setFrom] = useState("#2f7bff");
  const [to, setTo] = useState("#ff3da6");
  const [shift, setShift] = useState(0);
  const [base, setBase] = useState<readonly Rgb[] | null>(null);

  return (
    <div className="flex flex-wrap items-center gap-2">
      <Tooltip content={t("generators.rainbowHint")}>
        <Button size="sm" variant="secondary" onClick={() => onReplace(rainbowGrid(columns, rows))}>
          <Rainbow />
          {t("generators.rainbow")}
        </Button>
      </Tooltip>
      <Tooltip content={t("generators.harmonyHint")}>
        <Button size="sm" variant="secondary" onClick={() => onReplace(harmonyGrid(columns, rows))}>
          <Dices />
          {t("generators.harmony")}
        </Button>
      </Tooltip>

      <div className="glass-inset flex h-8 items-center gap-1.5 rounded-sm pr-1 pl-2">
        <ColorSwatchButton value={from} onChange={setFrom} label={t("generators.from")} />
        <ColorSwatchButton value={to} onChange={setTo} label={t("generators.to")} />
        <Button
          size="sm"
          variant="ghost"
          className="h-6 px-2"
          onClick={() => {
            const a = fromHex(from);
            const b = fromHex(to);
            if (a && b) onReplace(gradientGrid(a, b, columns, rows));
          }}
        >
          <Blend />
          {t("generators.gradient")}
        </Button>
      </div>

      <div className="glass-inset flex h-8 w-56 items-center gap-2 rounded-sm px-2.5">
        <SunMoon className="size-3.5 shrink-0 text-fg-subtle" />
        <Slider
          value={[shift]}
          min={-180}
          max={180}
          step={5}
          aria-label={t("generators.hueShift")}
          onValueChange={([v]) => {
            const snapshot = base ?? current;
            if (!base) setBase(current);
            setShift(v ?? 0);
            onReplace(hueShift(snapshot, v ?? 0));
          }}
          onValueCommit={() => {
            setBase(null);
            setShift(0);
          }}
        />
        <span className="w-9 text-right font-mono text-[10px] text-fg-muted tabular-nums">{shift}°</span>
      </div>

      <Button size="sm" variant="ghost" onClick={() => onReplace([...stock])} className="ml-auto">
        <RotateCcw />
        {t("generators.resetStock")}
      </Button>
    </div>
  );
}
