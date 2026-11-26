import { ChevronDown, RotateCcw } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Segmented } from "@/components/ui/segmented";
import { Select } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/cn";
import { DEFAULT_MATCH_OPTIONS, type MatchOptions } from "../api";
import { useRpOptions } from "../queries";

const STORAGE_KEY = "alxs.multi.matchOptions";
/** Select values standing for "no choice" (Select needs non-empty strings). */
const KEEP = "__keep";
const DEFAULT = "__default";
const PLAYER_OPTIONS = ["auto", "2", "4", "6", "8"] as const;

function load(): MatchOptions {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw
      ? { ...DEFAULT_MATCH_OPTIONS, ...(JSON.parse(raw) as Partial<MatchOptions>) }
      : DEFAULT_MATCH_OPTIONS;
  } catch {
    return DEFAULT_MATCH_OPTIONS;
  }
}

/** Match options remembered between sessions. */
export function useMatchOptions() {
  const [options, setOptions] = useState<MatchOptions>(load);
  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(options));
  }, [options]);
  return [options, setOptions] as const;
}

export function MatchOptionsPanel({
  value,
  onChange,
}: {
  value: MatchOptions;
  onChange: (next: MatchOptions) => void;
}) {
  const { t } = useTranslation("multi");
  const { data: catalog } = useRpOptions();
  const [open, setOpen] = useState(false);
  const [showMutators, setShowMutators] = useState(false);
  const set = (patch: Partial<MatchOptions>) => onChange({ ...value, ...patch });

  // Drop choices the current lists no longer offer (preset deleted, etc.).
  const preset = value.preset && catalog?.presets.includes(value.preset) ? value.preset : null;
  const modeName = catalog?.gameModes.find((m) => m.internal === value.gameMode)?.display;
  const changed = Object.keys(value.mutators).length;
  const summary = value.manual
    ? t("options.summaryManual")
    : [
        modeName ?? t("options.keepMode"),
        value.players ? t("options.playersN", { count: value.players }) : null,
        preset,
        changed ? t("options.mutatorsN", { count: changed }) : null,
      ]
        .filter(Boolean)
        .join(" · ");

  return (
    <div className="flex flex-col gap-2 rounded-[8px] border border-line p-2.5">
      <button
        type="button"
        className="flex w-full items-center justify-between gap-2 text-left"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        <span className="min-w-0">
          <span className="block text-xs font-medium text-fg">{t("options.title")}</span>
          <span className="block truncate text-xs text-fg-subtle">{summary}</span>
        </span>
        <ChevronDown
          className={cn("size-4 shrink-0 text-fg-subtle transition-transform", open && "rotate-180")}
        />
      </button>

      {open ? (
        <div className="flex flex-col gap-3 pt-1">
          <label className="flex items-start justify-between gap-3">
            <span className="min-w-0">
              <span className="block text-xs text-fg">{t("options.manual")}</span>
              <span className="block text-xs text-fg-subtle">{t("options.manualHint")}</span>
            </span>
            <Switch checked={value.manual} onCheckedChange={(manual) => set({ manual })} />
          </label>

          {value.manual || !catalog ? null : (
            <>
              <div className="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-2 text-xs">
                <span className="text-fg-subtle">{t("options.mode")}</span>
                <Select
                  value={value.gameMode ?? KEEP}
                  onValueChange={(v) => set({ gameMode: v === KEEP ? null : v })}
                  options={[
                    { value: KEEP, label: t("options.keepMode") },
                    ...catalog.gameModes.map((m) => ({ value: m.internal, label: m.display })),
                  ]}
                />
                <span className="text-fg-subtle">{t("options.players")}</span>
                <Segmented
                  size="sm"
                  value={value.players ? String(value.players) : "auto"}
                  onValueChange={(v) => set({ players: v === "auto" ? null : Number(v) })}
                  options={PLAYER_OPTIONS.map((p) => ({
                    value: p,
                    label: p === "auto" ? t("options.auto") : p,
                  }))}
                />
                <span className="text-fg-subtle">{t("options.preset")}</span>
                <Select
                  value={preset ?? KEEP}
                  onValueChange={(v) => set({ preset: v === KEEP ? null : v })}
                  options={[
                    { value: KEEP, label: t("options.noPreset") },
                    ...catalog.presets.map((p) => ({ value: p, label: p })),
                  ]}
                />
              </div>

              <div className="flex items-center justify-between gap-2">
                <Button size="sm" variant="ghost" onClick={() => setShowMutators((s) => !s)}>
                  <ChevronDown className={cn("transition-transform", showMutators && "rotate-180")} />
                  {t("options.mutators")}
                  {changed ? ` (${changed})` : null}
                </Button>
                {changed ? (
                  <Button size="sm" variant="ghost" onClick={() => set({ mutators: {} })}>
                    <RotateCcw />
                    {t("options.reset")}
                  </Button>
                ) : null}
              </div>
              {showMutators ? (
                <div className="grid max-h-72 grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-1.5 overflow-y-auto pr-1 text-xs">
                  {catalog.mutators.map((m) => (
                    <MutatorRow
                      key={m.category}
                      label={m.label}
                      values={m.values}
                      value={value.mutators[m.category]}
                      onChange={(v) => {
                        const next = { ...value.mutators };
                        if (v) next[m.category] = v;
                        else delete next[m.category];
                        set({ mutators: next });
                      }}
                    />
                  ))}
                </div>
              ) : null}
              <p className="text-xs text-fg-subtle">{t("options.hint")}</p>
            </>
          )}
        </div>
      ) : null}
    </div>
  );
}

function MutatorRow({
  label,
  values,
  value,
  onChange,
}: {
  label: string;
  values: { display: string; internal: string }[];
  value: string | undefined;
  onChange: (value: string | null) => void;
}) {
  return (
    <>
      <span className={cn("truncate", value ? "text-fg" : "text-fg-subtle")}>{label}</span>
      <Select
        value={value ?? DEFAULT}
        onValueChange={(v) => onChange(v === DEFAULT ? null : v)}
        options={values.map((v) => ({
          value: v.internal || DEFAULT,
          label: v.display || "Default",
        }))}
      />
    </>
  );
}
