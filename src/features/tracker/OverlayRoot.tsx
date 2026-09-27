import { invoke } from "@tauri-apps/api/core";
import { Flame, GripVertical, Snowflake } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { EVENTS, useTauriEvent } from "@/lib/events";
import { appConfigSchema } from "@/lib/game";
import { setLocale } from "@/lib/i18n";
import { arenaLabel, formatClock, type SessionState, sessionStateSchema, topMmrDelta } from "./api";
import { teamAverage } from "./components/LobbyPanel";

/**
 * Tracker HUD for the transparent always-on-top overlay window.
 * No router, no shell, no WebGL: it only listens to `tracker://session`
 * (plus one initial `tracker_session` call) and stays tiny.
 */
export function OverlayRoot() {
  const { t } = useTranslation("tracker");
  const [s, setS] = useState<SessionState | null>(null);

  useEffect(() => {
    for (const el of [document.documentElement, document.body]) {
      el.style.background = "transparent";
    }
    document.getElementById("root")?.style.setProperty("background", "transparent");
    invoke("tracker_session")
      .then((raw) => {
        const parsed = sessionStateSchema.safeParse(raw);
        if (parsed.success) setS(parsed.data);
      })
      .catch(() => {});
    invoke("config_get")
      .then((raw) => {
        const cfg = appConfigSchema.safeParse(raw);
        if (cfg.success) setLocale(cfg.data.locale);
      })
      .catch(() => {});
  }, []);

  useTauriEvent(EVENTS.trackerSession, sessionStateSchema, setS);

  const live = s?.live && !s.live.ended ? s.live : null;
  // Average MMR per team (lobby lookups), shown under the score.
  const blueAvg = teamAverage((s?.lobby ?? []).filter((p) => p.team === 0));
  const orangeAvg = teamAverage((s?.lobby ?? []).filter((p) => p.team === 1));
  const mmr = s ? topMmrDelta(s.mmr) : null;
  const streak = s?.streak ?? 0;

  return (
    <div
      data-tauri-drag-region
      className="flex h-screen w-screen select-none items-stretch overflow-hidden rounded-[10px] border border-white/10 bg-[rgb(10_11_15/0.82)] text-white backdrop-blur-xl"
    >
      <div data-tauri-drag-region className="grid w-5 place-items-center text-white/25">
        <GripVertical className="size-3.5" />
      </div>

      <div data-tauri-drag-region className="flex flex-1 items-center gap-4 py-3 pr-4">
        <div data-tauri-drag-region className="flex flex-col items-start">
          <span className="text-[11px] text-white/45">{t("overlay.session")}</span>
          <span className="text-[26px] font-semibold tabular-nums leading-none">
            <span className="text-[#3ddc97]">{s?.wins ?? 0}</span>
            <span className="mx-1 text-white/30">-</span>
            <span className="text-[#ff5d73]">{s?.losses ?? 0}</span>
          </span>
          <span
            className={cn(
              "mt-1 inline-flex items-center gap-1 text-xs font-medium",
              streak > 0 ? "text-[#ffbe8f]" : streak < 0 ? "text-[#9cc1ff]" : "text-white/40",
            )}
          >
            {streak > 0 ? <Flame className="size-3" /> : streak < 0 ? <Snowflake className="size-3" /> : null}
            {streak === 0 ? t("overlay.noStreak") : t("overlay.streak", { count: Math.abs(streak) })}
          </span>
        </div>

        <div data-tauri-drag-region className="h-12 w-px bg-white/10" />

        <div data-tauri-drag-region className="flex flex-1 flex-col items-center">
          <AnimatePresence mode="wait">
            {live ? (
              <motion.div
                key="live"
                initial={{ opacity: 0, y: 4 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -4 }}
                className="flex flex-col items-center"
              >
                <div className="flex items-center gap-3 text-[26px] font-semibold tabular-nums leading-none">
                  <span className="text-[#6aa5ff]">{live.blueScore}</span>
                  <span className="text-sm font-medium text-white/60">
                    {live.overtime ? "+" : ""}
                    {formatClock(live.timeSeconds)}
                  </span>
                  <span className="text-[#ff9a52]">{live.orangeScore}</span>
                </div>
                <span className="mt-1 flex items-center gap-2 text-[11px] text-white/45 tabular-nums">
                  {blueAvg != null ? <span className="text-[#9cc1ff]">{blueAvg}</span> : null}
                  <span>{arenaLabel(live.arena)}</span>
                  {orangeAvg != null ? <span className="text-[#ffbe8f]">{orangeAvg}</span> : null}
                </span>
              </motion.div>
            ) : (
              <motion.span
                key="idle"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="text-xs text-white/45"
              >
                {s?.connection === "connected" ? t("overlay.waiting") : t("overlay.notConnected")}
              </motion.span>
            )}
          </AnimatePresence>
        </div>

        {mmr ? (
          <>
            <div data-tauri-drag-region className="h-12 w-px bg-white/10" />
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
          </>
        ) : null}
      </div>
    </div>
  );
}
