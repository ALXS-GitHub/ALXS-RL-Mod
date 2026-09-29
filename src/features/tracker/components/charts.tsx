/**
 * Small SVG charts for the "Stats" tab (no chart library): stacked result
 * columns and a rolling-average line. Specs from the dataviz method: thin
 * marks, 4px rounded data ends, 2px surface gap, hairline grid, hover
 * tooltip on every mark, legend for two series. Colours validated for the
 * dark surface (lightness band, colour-blind separation, contrast).
 */
import { type ReactNode, useLayoutEffect, useRef, useState } from "react";
import { cn } from "@/lib/cn";

/** Wins / losses (validated pair, see file doc). */
export const WIN_COLOR = "#2aad76";
export const LOSS_COLOR = "#b23049";

const HEIGHT = 180;
const PAD = { top: 12, right: 8, bottom: 22, left: 34 };
const MAX_BAR = 24;
const GAP = 2;

function useWidth() {
  const ref = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => setWidth(Math.floor(entry?.contentRect.width ?? 0)));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return { ref, width };
}

/** Clean tick step for a max value (1, 2, 5 × 10ⁿ). */
function niceTicks(max: number, count = 4): number[] {
  if (max <= 0) return [0];
  const raw = max / count;
  const mag = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 5, 10].map((m) => m * mag).find((s) => s >= raw) ?? raw;
  const top = Math.ceil(max / step) * step;
  return Array.from({ length: Math.round(top / step) + 1 }, (_, i) => i * step);
}

function Grid({ ticks, y, width }: { ticks: number[]; y: (v: number) => number; width: number }) {
  return (
    <g>
      {ticks.map((v) => (
        <g key={v}>
          <line x1={PAD.left} x2={width - PAD.right} y1={y(v)} y2={y(v)} stroke="var(--color-line)" />
          <text
            x={PAD.left - 6}
            y={y(v)}
            textAnchor="end"
            dominantBaseline="middle"
            className="fill-[var(--color-fg-subtle)] text-[10px] tabular-nums"
          >
            {v}
          </text>
        </g>
      ))}
    </g>
  );
}

function Tooltip({ x, width, children }: { x: number; width: number; children: ReactNode }) {
  // Beside the hovered mark, never over it: right of it, or left near the edge.
  const right = x + 180 < width;
  return (
    <div
      className="pointer-events-none absolute top-0 z-10 rounded-md border border-line-strong bg-[var(--color-surface-strong)] px-2.5 py-1.5 text-xs shadow-lg"
      style={right ? { left: x + 16 } : { left: x - 16, transform: "translateX(-100%)" }}
    >
      {children}
    </div>
  );
}

/** Legend / tooltip key: a swatch (bars) or a short stroke (lines). */
export function Key({ color, shape = "rect" }: { color: string; shape?: "rect" | "line" }) {
  return shape === "rect" ? (
    <span className="inline-block size-2.5 rounded-[2px]" style={{ background: color }} />
  ) : (
    <span className="inline-block h-0.5 w-3 rounded-full" style={{ background: color }} />
  );
}

export interface ColumnDatum {
  key: string;
  label: string;
  wins: number;
  losses: number;
}

/** Wins (bottom) and losses stacked per period. */
export function ResultColumns({
  data,
  winsLabel,
  lossesLabel,
  ariaLabel,
}: {
  data: ColumnDatum[];
  winsLabel: string;
  lossesLabel: string;
  ariaLabel: string;
}) {
  const { ref, width } = useWidth();
  const [hover, setHover] = useState<number | null>(null);
  const max = Math.max(1, ...data.map((d) => d.wins + d.losses));
  const ticks = niceTicks(max);
  const top = ticks[ticks.length - 1] ?? max;
  const plotW = Math.max(0, width - PAD.left - PAD.right);
  const slot = data.length ? plotW / data.length : 0;
  const bar = Math.max(2, Math.min(MAX_BAR, slot - GAP * 2));
  const y = (v: number) => PAD.top + (HEIGHT - PAD.top - PAD.bottom) * (1 - v / top);
  const h = (v: number) => (HEIGHT - PAD.top - PAD.bottom) * (v / top);
  const labelEvery = Math.max(1, Math.ceil(data.length / 6));
  const hovered = hover !== null ? data[hover] : undefined;

  return (
    <div ref={ref} className="relative">
      {width > 0 ? (
        <svg
          width={width}
          height={HEIGHT}
          role="img"
          aria-label={ariaLabel}
          onPointerLeave={() => setHover(null)}
        >
          <Grid ticks={ticks} y={y} width={width} />
          {data.map((d, i) => {
            const cx = PAD.left + slot * i + slot / 2;
            const x = cx - bar / 2;
            const winH = h(d.wins);
            const lossH = h(d.losses);
            const base = HEIGHT - PAD.bottom;
            const r = Math.min(4, bar / 2);
            const segment = (y0: number, height: number, color: string, rounded: boolean) =>
              height <= 0 ? null : rounded ? (
                <path
                  d={`M${x},${y0 + height} V${y0 + r} Q${x},${y0} ${x + r},${y0} H${x + bar - r} Q${x + bar},${y0} ${x + bar},${y0 + r} V${y0 + height} Z`}
                  fill={color}
                />
              ) : (
                <rect x={x} y={y0} width={bar} height={height} fill={color} />
              );
            const lossTop = base - winH - (d.wins && d.losses ? GAP : 0) - lossH;
            return (
              <g key={d.key} opacity={hover === null || hover === i ? 1 : 0.55}>
                {segment(base - winH, winH, WIN_COLOR, d.losses === 0)}
                {segment(lossTop, lossH, LOSS_COLOR, true)}
                {i % labelEvery === 0 ? (
                  <text
                    x={cx}
                    y={HEIGHT - 6}
                    textAnchor="middle"
                    className="fill-[var(--color-fg-subtle)] text-[10px]"
                  >
                    {d.label}
                  </text>
                ) : null}
                {/* Hit target: the whole slot, bigger than the mark. */}
                <rect
                  x={PAD.left + slot * i}
                  y={PAD.top}
                  width={slot}
                  height={HEIGHT - PAD.top - PAD.bottom}
                  fill="transparent"
                  role="button"
                  tabIndex={0}
                  aria-label={`${d.label}: ${d.wins} ${winsLabel}, ${d.losses} ${lossesLabel}`}
                  onPointerEnter={() => setHover(i)}
                  onFocus={() => setHover(i)}
                  onBlur={() => setHover(null)}
                />
              </g>
            );
          })}
        </svg>
      ) : (
        <div style={{ height: HEIGHT }} />
      )}
      {hovered && hover !== null ? (
        <Tooltip x={PAD.left + slot * hover + slot / 2} width={width}>
          <p className="mb-1 text-fg-subtle">{hovered.label}</p>
          <p className="flex items-center gap-1.5">
            <Key color={WIN_COLOR} shape="line" />
            <b className="tabular-nums">{hovered.wins}</b>
            <span className="text-fg-muted">{winsLabel}</span>
          </p>
          <p className="flex items-center gap-1.5">
            <Key color={LOSS_COLOR} shape="line" />
            <b className="tabular-nums">{hovered.losses}</b>
            <span className="text-fg-muted">{lossesLabel}</span>
          </p>
        </Tooltip>
      ) : null}
    </div>
  );
}

export interface LinePoint {
  key: string;
  label: string;
  value: number;
  /** Extra tooltip rows (label, value). */
  details: [string, string][];
}

/** One series over time: 2px line, 10% area wash, crosshair tooltip. */
export function TrendLine({ data, ariaLabel }: { data: LinePoint[]; ariaLabel: string }) {
  const { ref, width } = useWidth();
  const [hover, setHover] = useState<number | null>(null);
  const color = "var(--color-accent)";
  const max = Math.max(1, ...data.map((d) => d.value));
  const ticks = niceTicks(max);
  const top = ticks[ticks.length - 1] ?? max;
  const plotW = Math.max(0, width - PAD.left - PAD.right);
  const x = (i: number) => PAD.left + (data.length > 1 ? (plotW * i) / (data.length - 1) : plotW / 2);
  const y = (v: number) => PAD.top + (HEIGHT - PAD.top - PAD.bottom) * (1 - v / top);
  const line = data.map((d, i) => `${i ? "L" : "M"}${x(i)},${y(d.value)}`).join(" ");
  const area = data.length
    ? `${line} L${x(data.length - 1)},${HEIGHT - PAD.bottom} L${x(0)},${HEIGHT - PAD.bottom} Z`
    : "";
  const hovered = hover !== null ? data[hover] : undefined;
  const last = data[data.length - 1];

  const onMove = (e: React.PointerEvent<SVGSVGElement>) => {
    if (!data.length) return;
    const box = e.currentTarget.getBoundingClientRect();
    const px = e.clientX - box.left;
    const i = data.length > 1 ? Math.round(((px - PAD.left) / plotW) * (data.length - 1)) : 0;
    setHover(Math.min(data.length - 1, Math.max(0, i)));
  };

  return (
    <div ref={ref} className="relative">
      {width > 0 ? (
        <svg
          width={width}
          height={HEIGHT}
          role="img"
          aria-label={ariaLabel}
          onPointerMove={onMove}
          onPointerLeave={() => setHover(null)}
        >
          <Grid ticks={ticks} y={y} width={width} />
          <path d={area} fill={color} opacity={0.1} />
          <path
            d={line}
            fill="none"
            stroke={color}
            strokeWidth={2}
            strokeLinejoin="round"
            strokeLinecap="round"
          />
          {hovered && hover !== null ? (
            <>
              <line
                x1={x(hover)}
                x2={x(hover)}
                y1={PAD.top}
                y2={HEIGHT - PAD.bottom}
                stroke="var(--color-line-strong)"
              />
              <circle
                cx={x(hover)}
                cy={y(hovered.value)}
                r={4}
                fill={color}
                stroke="#111218"
                strokeWidth={2}
              />
            </>
          ) : last ? (
            <circle
              cx={x(data.length - 1)}
              cy={y(last.value)}
              r={4}
              fill={color}
              stroke="#111218"
              strokeWidth={2}
            />
          ) : null}
        </svg>
      ) : (
        <div style={{ height: HEIGHT }} />
      )}
      {hovered && hover !== null ? (
        <Tooltip x={x(hover)} width={width}>
          <p className="mb-1 text-fg-subtle">{hovered.label}</p>
          {hovered.details.map(([k, v]) => (
            <p key={k} className={cn("flex items-center gap-1.5")}>
              <b className="tabular-nums">{v}</b>
              <span className="text-fg-muted">{k}</span>
            </p>
          ))}
        </Tooltip>
      ) : null}
    </div>
  );
}
