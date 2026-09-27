import { create } from "zustand";

export type FxQuality = "off" | "low" | "high";

/** Three accent colours driving both the aurora shader and CSS tokens. */
export type AccentTriplet = readonly [string, string, string];

export const DEFAULT_ACCENTS: AccentTriplet = ["#7c6cff", "#2f7bff", "#ff7a1a"];

interface FxState {
  quality: FxQuality;
  /** Why rendering is paused, if it is (game focused, window hidden…). */
  pauseReasons: ReadonlySet<string>;
  accents: AccentTriplet;
  setQuality: (q: FxQuality) => void;
  setPaused: (reason: string, paused: boolean) => void;
  setAccents: (accents: AccentTriplet) => void;
  resetAccents: () => void;
}

function applyCssAccents([a, b, c]: AccentTriplet) {
  const root = document.documentElement.style;
  root.setProperty("--accent", a);
  root.setProperty("--accent-2", b);
  root.setProperty("--accent-3", c);
}

export const useFx = create<FxState>((set) => ({
  quality: "high",
  pauseReasons: new Set(),
  accents: DEFAULT_ACCENTS,
  setQuality: (quality) => set({ quality }),
  setPaused: (reason, paused) =>
    set((s) => {
      const next = new Set(s.pauseReasons);
      if (paused) next.add(reason);
      else next.delete(reason);
      return { pauseReasons: next };
    }),
  setAccents: (accents) => {
    applyCssAccents(accents);
    set({ accents });
  },
  resetAccents: () => {
    applyCssAccents(DEFAULT_ACCENTS);
    set({ accents: DEFAULT_ACCENTS });
  },
}));

export const selectFxRunning = (s: FxState) => s.quality !== "off" && s.pauseReasons.size === 0;
