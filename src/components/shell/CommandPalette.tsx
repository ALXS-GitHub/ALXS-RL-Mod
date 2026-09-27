import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useNavigate } from "@tanstack/react-router";
import { Command } from "cmdk";
import { Languages, Search, Sparkles, WandSparkles } from "lucide-react";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { NAV } from "@/app/nav";
import { Kbd } from "@/components/ui/feedback";
import { useConfig, useUpdateConfig } from "@/lib/game";
import { useUi } from "@/stores/ui";

/** Ctrl+K launcher: jump to any page or run a global action. */
export function CommandPalette() {
  const { t } = useTranslation();
  const open = useUi((s) => s.commandOpen);
  const setOpen = useUi((s) => s.setCommandOpen);
  const navigate = useNavigate();
  const { data: config } = useConfig();
  const updateConfig = useUpdateConfig();

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen(!useUi.getState().commandOpen);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setOpen]);

  const run = (fn: () => void) => {
    setOpen(false);
    fn();
  };

  const itemClass =
    "flex h-10 cursor-pointer items-center gap-3 rounded-md px-3 text-sm text-fg-muted data-[selected=true]:bg-white/8 data-[selected=true]:text-fg [&_svg]:size-4";

  return (
    <DialogPrimitive.Root open={open} onOpenChange={setOpen}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-[80] bg-black/50 backdrop-blur-sm" />
        <DialogPrimitive.Content className="glass-strong fixed top-[18%] left-1/2 z-[80] w-[min(640px,calc(100vw-2rem))] -translate-x-1/2 overflow-hidden rounded-xl outline-none">
          <DialogPrimitive.Title className="sr-only">{t("titlebar.search")}</DialogPrimitive.Title>
          <DialogPrimitive.Description className="sr-only">
            {t("palette.placeholder")}
          </DialogPrimitive.Description>
          <Command loop className="flex flex-col">
            <div className="flex items-center gap-3 border-line border-b px-4">
              <Search className="size-4 text-fg-subtle" />
              <Command.Input
                autoFocus
                placeholder={t("palette.placeholder")}
                className="h-14 flex-1 bg-transparent text-base text-fg outline-none placeholder:text-fg-subtle"
              />
              <Kbd>Esc</Kbd>
            </div>
            <Command.List className="max-h-[380px] overflow-y-auto p-2">
              <Command.Empty className="px-3 py-8 text-center text-sm text-fg-subtle">
                {t("palette.noResults")}
              </Command.Empty>
              <Command.Group
                heading={t("palette.pages")}
                className="[&_[cmdk-group-heading]]:px-3 [&_[cmdk-group-heading]]:py-2 [&_[cmdk-group-heading]]:text-[10px] [&_[cmdk-group-heading]]:font-semibold [&_[cmdk-group-heading]]:uppercase [&_[cmdk-group-heading]]:tracking-[0.2em] [&_[cmdk-group-heading]]:text-fg-subtle"
              >
                {NAV.map((item) => {
                  const Icon = item.icon;
                  return (
                    <Command.Item
                      key={item.to}
                      value={`${t(item.labelKey)} ${item.keywords.join(" ")}`}
                      onSelect={() => run(() => void navigate({ to: item.to }))}
                      className={itemClass}
                    >
                      <Icon />
                      {t(item.labelKey)}
                    </Command.Item>
                  );
                })}
              </Command.Group>
              <Command.Group
                heading={t("palette.actions")}
                className="[&_[cmdk-group-heading]]:px-3 [&_[cmdk-group-heading]]:py-2 [&_[cmdk-group-heading]]:text-[10px] [&_[cmdk-group-heading]]:font-semibold [&_[cmdk-group-heading]]:uppercase [&_[cmdk-group-heading]]:tracking-[0.2em] [&_[cmdk-group-heading]]:text-fg-subtle"
              >
                <Command.Item
                  value="language langue english français"
                  onSelect={() =>
                    run(() => updateConfig.mutate({ locale: config?.locale === "fr" ? "en" : "fr" }))
                  }
                  className={itemClass}
                >
                  <Languages />
                  {config?.locale === "fr" ? "Switch to English" : "Passer en français"}
                </Command.Item>
                <Command.Item
                  value="effects effets shader aurora"
                  onSelect={() =>
                    run(() =>
                      updateConfig.mutate({ fxQuality: config?.fxQuality === "off" ? "high" : "off" }),
                    )
                  }
                  className={itemClass}
                >
                  {config?.fxQuality === "off" ? <Sparkles /> : <WandSparkles />}
                  {config?.fxQuality === "off" ? t("palette.fxOn") : t("palette.fxOff")}
                </Command.Item>
              </Command.Group>
            </Command.List>
          </Command>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
