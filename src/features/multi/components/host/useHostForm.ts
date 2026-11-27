import { useEffect, useState } from "react";
import { DEFAULT_MATCH_OPTIONS, type HostPreset, type MatchOptions } from "../../api";

/** Everything the host tab sends: map, arena replaced, match options. */
export interface HostForm {
  mapId: string | null;
  /** `null` = the map's preferred arena (else Underpass). */
  target: string | null;
  options: MatchOptions;
  /** Saved preset this form came from (for "update preset"). */
  presetId: string | null;
}

const STORAGE_KEY = "alxs.multi.hostForm";
/** Match options saved by the previous version of the host card. */
const LEGACY_OPTIONS_KEY = "alxs.multi.matchOptions";

const EMPTY: HostForm = { mapId: null, target: null, options: DEFAULT_MATCH_OPTIONS, presetId: null };

function load(): HostForm {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<HostForm>;
      return { ...EMPTY, ...parsed, options: { ...DEFAULT_MATCH_OPTIONS, ...parsed.options } };
    }
    const legacy = localStorage.getItem(LEGACY_OPTIONS_KEY);
    if (legacy) {
      return {
        ...EMPTY,
        options: { ...DEFAULT_MATCH_OPTIONS, ...(JSON.parse(legacy) as Partial<MatchOptions>) },
      };
    }
  } catch {
    // Corrupted storage: start fresh.
  }
  return EMPTY;
}

/** Host form remembered between sessions. */
export function useHostForm() {
  const [form, setForm] = useState<HostForm>(load);
  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(form));
  }, [form]);
  const patch = (p: Partial<HostForm>) => setForm((f) => ({ ...f, ...p }));
  const patchOptions = (p: Partial<MatchOptions>) =>
    setForm((f) => ({ ...f, options: { ...f.options, ...p } }));
  const applyPreset = (preset: HostPreset) =>
    setForm({ mapId: preset.mapId, target: preset.target, options: preset.options, presetId: preset.id });
  return { form, patch, patchOptions, applyPreset };
}
