import * as PopoverPrimitive from "@radix-ui/react-popover";
import * as SliderPrimitive from "@radix-ui/react-slider";
import { type KeyboardEvent, type PointerEvent, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/cn";
import { type Hsv, hexToHsv, hsvToHex, normalizeHex } from "@/lib/color";

export interface ColorPreset {
  hex: string;
  label: string;
}

interface ColorPickerProps {
  /** `#rrggbb`. */
  value: string;
  onChange: (hex: string) => void;
  presets?: readonly ColorPreset[];
  /** Wheel diameter in px. */
  size?: number;
  className?: string;
}

const HUE_RING = "conic-gradient(from 90deg, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00)";

/**
 * Colour wheel (hue around, saturation outwards), an intensity slider, a
 * hex field and optional presets. Shared by the palette editor and the
 * item colour options.
 */
export function ColorPicker({ value, onChange, presets, size = 176, className }: ColorPickerProps) {
  const { t } = useTranslation();
  const [hsv, setHsv] = useState<Hsv>(() => hexToHsv(value) ?? { h: 0, s: 0, v: 1 });
  const [draft, setDraft] = useState(value);
  const wheel = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);

  // Follow outside changes, keeping the hue of greys (it is lost in hex).
  useEffect(() => {
    setDraft(value);
    setHsv((cur) => {
      if (hsvToHex(cur) === normalizeHex(value)) return cur;
      const next = hexToHsv(value);
      if (!next) return cur;
      return next.s === 0 || next.v === 0 ? { ...next, h: cur.h } : next;
    });
  }, [value]);

  const commit = (next: Hsv) => {
    setHsv(next);
    const hex = hsvToHex(next);
    setDraft(hex);
    onChange(hex);
  };

  const fromPointer = (e: PointerEvent<HTMLDivElement>) => {
    const el = wheel.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const r = rect.width / 2;
    const dx = e.clientX - (rect.left + r);
    const dy = e.clientY - (rect.top + r);
    const h = ((Math.atan2(dy, dx) * 180) / Math.PI + 360) % 360;
    const s = Math.min(1, Math.hypot(dx, dy) / r);
    commit({ h, s, v: hsv.v === 0 ? 1 : hsv.v });
  };

  const onWheelKey = (e: KeyboardEvent<HTMLDivElement>) => {
    const step = e.shiftKey ? 15 : 5;
    const moves: Record<string, Hsv> = {
      ArrowLeft: { ...hsv, h: (hsv.h - step + 360) % 360 },
      ArrowRight: { ...hsv, h: (hsv.h + step) % 360 },
      ArrowUp: { ...hsv, s: Math.min(1, hsv.s + 0.05) },
      ArrowDown: { ...hsv, s: Math.max(0, hsv.s - 0.05) },
    };
    const next = moves[e.key];
    if (next) {
      e.preventDefault();
      commit(next);
    }
  };

  const rad = (hsv.h * Math.PI) / 180;
  const thumbX = 50 + Math.cos(rad) * hsv.s * 50;
  const thumbY = 50 + Math.sin(rad) * hsv.s * 50;
  const current = hsvToHex(hsv);
  const brightest = hsvToHex({ ...hsv, v: 1 });

  return (
    <div className={cn("flex flex-col gap-3", className)}>
      <div className="flex items-start gap-4">
        <div
          ref={wheel}
          role="slider"
          tabIndex={0}
          aria-label={t("colorPicker.wheel")}
          aria-valuetext={`${Math.round(hsv.h)}°, ${Math.round(hsv.s * 100)}%`}
          aria-valuenow={Math.round(hsv.h)}
          aria-valuemin={0}
          aria-valuemax={359}
          onKeyDown={onWheelKey}
          onPointerDown={(e) => {
            dragging.current = true;
            e.currentTarget.setPointerCapture(e.pointerId);
            fromPointer(e);
          }}
          onPointerMove={(e) => dragging.current && fromPointer(e)}
          onPointerUp={() => {
            dragging.current = false;
          }}
          className="relative shrink-0 cursor-crosshair touch-none rounded-full outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-accent)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--color-bg)]"
          style={{ width: size, height: size }}
        >
          <div className="absolute inset-0 rounded-full" style={{ background: HUE_RING }} />
          <div
            className="absolute inset-0 rounded-full"
            style={{ background: "radial-gradient(circle closest-side, #fff, transparent)" }}
          />
          <div className="absolute inset-0 rounded-full bg-black" style={{ opacity: 1 - hsv.v }} />
          <div className="absolute inset-0 rounded-full shadow-[inset_0_0_0_1px_rgb(255_255_255/0.12)]" />
          <span
            className="pointer-events-none absolute size-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_1px_4px_rgb(0_0_0/0.6)]"
            style={{ left: `${thumbX}%`, top: `${thumbY}%`, background: current }}
          />
        </div>
        <div className="flex min-w-0 flex-1 flex-col gap-3">
          <div
            className="h-12 w-full rounded-[8px] shadow-[inset_0_0_0_1px_rgb(255_255_255/0.12)]"
            style={{ background: current }}
          />
          <label className="flex flex-col gap-1">
            <span className="text-xs text-fg-subtle">{t("colorPicker.hex")}</span>
            <Input
              value={draft}
              spellCheck={false}
              className="font-mono uppercase"
              onChange={(e) => {
                setDraft(e.target.value);
                const hex = normalizeHex(e.target.value);
                if (hex) {
                  const next = hexToHsv(hex);
                  if (next) setHsv(next.s === 0 || next.v === 0 ? { ...next, h: hsv.h } : next);
                  onChange(hex);
                }
              }}
              onBlur={() => setDraft(current)}
            />
          </label>
        </div>
      </div>
      <div className="flex flex-col gap-1.5">
        <span className="flex justify-between text-xs text-fg-subtle">
          {t("colorPicker.intensity")}
          <span className="tabular-nums">{Math.round(hsv.v * 100)}%</span>
        </span>
        <SliderPrimitive.Root
          value={[Math.round(hsv.v * 100)]}
          min={0}
          max={100}
          step={1}
          onValueChange={([v]) => commit({ ...hsv, v: (v ?? 100) / 100 })}
          className="relative flex h-5 w-full touch-none select-none items-center"
          aria-label={t("colorPicker.intensity")}
        >
          <SliderPrimitive.Track
            className="relative h-2 grow rounded-full shadow-[inset_0_0_0_1px_rgb(255_255_255/0.1)]"
            style={{ background: `linear-gradient(to right, #000, ${brightest})` }}
          />
          <SliderPrimitive.Thumb className="block size-4 rounded-full border-2 border-white bg-transparent shadow-[0_1px_3px_rgb(0_0_0/0.6)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-accent)]" />
        </SliderPrimitive.Root>
      </div>
      {presets?.length ? (
        <div className="flex flex-wrap gap-1.5">
          {presets.map((p) => {
            const selected = normalizeHex(p.hex) === normalizeHex(value);
            return (
              <button
                key={p.hex}
                type="button"
                title={p.label}
                aria-label={p.label}
                aria-pressed={selected}
                onClick={() => {
                  const next = hexToHsv(p.hex);
                  if (next) commit(next);
                }}
                className={cn(
                  "size-6 rounded-full border border-white/10 transition-shadow duration-150",
                  selected && "shadow-[0_0_0_2px_var(--color-bg),0_0_0_3.5px_var(--color-accent)]",
                )}
                style={{ background: p.hex }}
              />
            );
          })}
        </div>
      ) : null}
    </div>
  );
}

interface ColorSwatchButtonProps {
  value: string;
  onChange: (hex: string) => void;
  /** Accessible name (also the tooltip). */
  label: string;
  presets?: readonly ColorPreset[];
  className?: string;
}

/** A colour swatch that opens the colour picker in a popover. */
export function ColorSwatchButton({ value, onChange, label, presets, className }: ColorSwatchButtonProps) {
  return (
    <PopoverPrimitive.Root>
      <PopoverPrimitive.Trigger asChild>
        <button
          type="button"
          title={label}
          aria-label={label}
          className={cn(
            "size-5 shrink-0 cursor-pointer rounded-[5px] shadow-[inset_0_0_0_1px_rgb(255_255_255/0.18)] outline-none transition-shadow focus-visible:ring-2 focus-visible:ring-[var(--color-accent)]",
            className,
          )}
          style={{ background: value }}
        />
      </PopoverPrimitive.Trigger>
      <PopoverPrimitive.Portal>
        <PopoverPrimitive.Content
          sideOffset={8}
          align="start"
          collisionPadding={12}
          className="glass-strong z-[70] w-[320px] rounded-lg p-3 outline-none"
        >
          <ColorPicker value={value} onChange={onChange} presets={presets} size={140} />
        </PopoverPrimitive.Content>
      </PopoverPrimitive.Portal>
    </PopoverPrimitive.Root>
  );
}
