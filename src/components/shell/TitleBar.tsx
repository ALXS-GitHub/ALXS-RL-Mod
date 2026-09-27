import { getCurrentWindow } from "@tauri-apps/api/window";
import { Command, Copy, Minus, Square, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import appLogo from "@/assets/alxs-rl-mod-logo-source.png";
import { Kbd } from "@/components/ui/feedback";
import { isTauri } from "@/lib/ipc";
import { useUi } from "@/stores/ui";
import { GameStatusPill } from "./GameStatusPill";

/** The window has no native decorations: this is the whole title bar. */
export function TitleBar() {
  const { t } = useTranslation();
  const setCommandOpen = useUi((s) => s.setCommandOpen);
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!isTauri) return;
    const win = getCurrentWindow();
    let unlisten: (() => void) | undefined;
    void win.isMaximized().then(setMaximized);
    void win
      .onResized(() => {
        void win.isMaximized().then(setMaximized);
      })
      .then((fn) => {
        unlisten = fn;
      });
    return () => unlisten?.();
  }, []);

  const win = isTauri ? getCurrentWindow() : null;

  return (
    <header
      data-tauri-drag-region
      className="drag relative z-40 flex h-10 shrink-0 items-center gap-3 border-line border-b bg-[rgb(10_11_15/0.6)] pl-3.5 backdrop-blur-xl"
    >
      <div data-tauri-drag-region className="flex items-center gap-2.5">
        <img src={appLogo} alt="" className="size-6 shrink-0 object-contain" />
        <span data-tauri-drag-region className="text-[13px] font-semibold">
          {t("app.name")}
        </span>
      </div>

      <div data-tauri-drag-region className="flex flex-1 justify-center">
        <button
          type="button"
          onClick={() => setCommandOpen(true)}
          className="no-drag flex h-7 w-full max-w-sm items-center gap-2 rounded-[7px] border border-line bg-white/[0.04] px-2.5 text-xs text-fg-subtle transition-colors hover:border-line-strong hover:text-fg-muted"
        >
          <Command className="size-3.5" />
          <span className="flex-1 text-left">{t("titlebar.search")}</span>
          <Kbd>Ctrl</Kbd>
          <Kbd>K</Kbd>
        </button>
      </div>

      <div className="no-drag flex items-center gap-3">
        <GameStatusPill />
        <div className="flex h-10 items-stretch">
          <button
            type="button"
            aria-label={t("titlebar.minimize")}
            onClick={() => void win?.minimize()}
            className="grid w-11 place-items-center text-fg-muted hover:bg-white/[0.07] hover:text-fg"
          >
            <Minus className="size-4" />
          </button>
          <button
            type="button"
            aria-label={maximized ? t("titlebar.restore") : t("titlebar.maximize")}
            onClick={() => void win?.toggleMaximize()}
            className="grid w-11 place-items-center text-fg-muted hover:bg-white/[0.07] hover:text-fg"
          >
            {maximized ? <Copy className="size-3.5" /> : <Square className="size-3.5" />}
          </button>
          <button
            type="button"
            aria-label={t("titlebar.close")}
            onClick={() => void win?.close()}
            className="grid w-12 place-items-center text-fg-muted hover:bg-danger hover:text-white"
          >
            <X className="size-4" />
          </button>
        </div>
      </div>
    </header>
  );
}
