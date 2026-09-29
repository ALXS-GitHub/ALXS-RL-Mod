import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { cn } from "@/lib/cn";
import { useTauriEvent } from "@/lib/events";
import { PERSONAL_EVENTS, type RatingsState, ratingsStateSchema, teamAverage, topMmrDelta } from "../api";

/**
 * Overlay block of the personal edition: team MMR averages of the live
 * match and your most-moved playlist. The overlay has no query client, so
 * this keeps its own state (one `ratings_state` call + `personal://ratings`).
 */
export function OverlayRatings({ live }: { live: boolean }) {
  const [r, setR] = useState<RatingsState | null>(null);

  useEffect(() => {
    invoke("ratings_state")
      .then((raw) => {
        const parsed = ratingsStateSchema.safeParse(raw);
        if (parsed.success) setR(parsed.data);
      })
      .catch(() => {});
  }, []);
  useTauriEvent(PERSONAL_EVENTS.ratings, ratingsStateSchema, setR);

  if (!r) return null;
  const blueAvg = live ? teamAverage(r.lobby.filter((p) => p.team === 0)) : null;
  const orangeAvg = live ? teamAverage(r.lobby.filter((p) => p.team === 1)) : null;
  const mmr = topMmrDelta(r.mmr);
  if (blueAvg == null && orangeAvg == null && !mmr) return null;

  return (
    <>
      <div data-tauri-drag-region className="h-12 w-px bg-white/10" />
      <div data-tauri-drag-region className="flex items-center gap-3">
        {blueAvg != null || orangeAvg != null ? (
          <div
            data-tauri-drag-region
            className="flex flex-col items-end text-[11px] tabular-nums leading-tight"
          >
            <span className="text-[#9cc1ff]">{blueAvg ?? "—"}</span>
            <span className="text-[#ffbe8f]">{orangeAvg ?? "—"}</span>
          </div>
        ) : null}
        {mmr ? (
          <div data-tauri-drag-region className="flex items-center gap-2">
            {mmr.tierIcon ? <img src={mmr.tierIcon} alt="" className="size-9 object-contain" /> : null}
            <div className="flex flex-col items-end">
              <span className="text-lg font-semibold tabular-nums leading-none">{mmr.latest}</span>
              <span
                className={cn(
                  "text-xs font-semibold tabular-nums",
                  mmr.delta > 0 ? "text-[#3ddc97]" : mmr.delta < 0 ? "text-[#ff5d73]" : "text-white/40",
                )}
              >
                {mmr.delta > 0 ? "+" : ""}
                {mmr.delta} MMR
              </span>
            </div>
          </div>
        ) : null}
      </div>
    </>
  );
}
