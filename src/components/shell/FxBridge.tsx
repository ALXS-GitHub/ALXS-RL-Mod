import { useEffect } from "react";
import { useConfig, useGameStatus } from "@/lib/game";
import { setLocale } from "@/lib/i18n";
import { useFx } from "@/stores/fx";

/**
 * Keeps the visual layer in sync with the rest of the app:
 * - quality + locale from the persisted config,
 * - pause while Rocket League has focus (if enabled),
 * - pause while the window is hidden/minimised.
 */
export function FxBridge() {
  const { data: config } = useConfig();
  const { data: game } = useGameStatus();
  const setQuality = useFx((s) => s.setQuality);
  const setPaused = useFx((s) => s.setPaused);

  useEffect(() => {
    if (config) setQuality(config.fxQuality);
  }, [config?.fxQuality, config, setQuality]);

  useEffect(() => {
    if (config) setLocale(config.locale);
  }, [config?.locale, config]);

  const gameFocused = Boolean(game?.running.foreground && (config?.fxPauseWhenGameFocused ?? true));
  useEffect(() => {
    setPaused("game-focused", gameFocused);
  }, [gameFocused, setPaused]);

  useEffect(() => {
    const onVisibility = () => setPaused("hidden", document.hidden);
    onVisibility();
    document.addEventListener("visibilitychange", onVisibility);
    return () => document.removeEventListener("visibilitychange", onVisibility);
  }, [setPaused]);

  return null;
}
