import { invoke } from "@tauri-apps/api/core";
import { Flame, GripVertical, Snowflake } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { lazy, Suspense, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { EVENTS, useTauriEvent } from "@/lib/events";
import { appConfigSchema } from "@/lib/game";
import { setLocale } from "@/lib/i18n";
import { arenaLabel, formatClock, type SessionState, sessionStateSchema } from "./api";

// Personal edition only (docs/PERSONAL_EDITION.md): team MMR and your MMR.
const OverlayRatings = __ALXS_PERSONAL__
  ? lazy(() => import("@/personal/tracker").then((m) => ({ default: m.OverlayRatings })))
  : null;

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
                <span className="mt-1 text-[11px] text-white/45">{arenaLabel(live.arena)}</span>
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

        {OverlayRatings ? (
          <Suspense fallback={null}>
            <OverlayRatings live={Boolean(live)} />
          </Suspense>
        ) : null}
      </div>
    </div>
  );
}
