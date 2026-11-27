import { openUrl } from "@tauri-apps/plugin-opener";
import { BookOpen, ChevronDown, ExternalLink } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { GlassPanel } from "@/components/ui/glass";
import { notify } from "@/components/ui/toast";
import { cn } from "@/lib/cn";
import { BAKKESMOD_DOWNLOAD, ROCKET_PLUGIN_PAGE, TAILSCALE_DOWNLOAD } from "../api";

/** Steps of the guide; each has `title` and `body` (lines split on `\n`). */
const STEPS = [
  { key: "tailscale", link: TAILSCALE_DOWNLOAD },
  { key: "bakkes", link: BAKKESMOD_DOWNLOAD },
  { key: "rocketPlugin", link: ROCKET_PLUGIN_PAGE },
  { key: "app", link: null },
  { key: "maps", link: null },
  { key: "play", link: null },
  { key: "trouble", link: null },
] as const;

/** Step-by-step setup for every player, folded by default. */
export function GuidePanel({ defaultOpen = false }: { defaultOpen?: boolean }) {
  const { t } = useTranslation("multi");
  const [open, setOpen] = useState(defaultOpen);

  return (
    <GlassPanel className="flex flex-col gap-3 p-4">
      <button
        type="button"
        className="flex w-full items-center justify-between gap-3 text-left"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        <span className="flex items-center gap-2">
          <BookOpen className="size-4 text-fg-subtle" />
          <span className="text-sm font-semibold">{t("guide.title")}</span>
          <span className="text-xs text-fg-subtle">{t("guide.subtitle")}</span>
        </span>
        <ChevronDown className={cn("size-4 text-fg-subtle transition-transform", open && "rotate-180")} />
      </button>
      {open ? (
        <ol className="flex flex-col gap-4">
          {STEPS.map((step, i) => (
            <li key={step.key} className="flex flex-col gap-1.5 border-line border-t pt-3">
              <div className="flex items-center justify-between gap-3">
                <h3 className="text-[13px] font-medium">
                  <span className="mr-2 text-fg-subtle tabular-nums">{i + 1}.</span>
                  {t(`guide.steps.${step.key}.title`)}
                </h3>
                {step.link ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => step.link && void openUrl(step.link).catch(notify.error)}
                  >
                    <ExternalLink />
                    {t(`guide.steps.${step.key}.link`)}
                  </Button>
                ) : null}
              </div>
              <ul className="flex flex-col gap-1 text-xs text-fg-muted">
                {t(`guide.steps.${step.key}.body`)
                  .split("\n")
                  .map((line) => (
                    <li key={line} className="flex items-start gap-2">
                      <span className="mt-1.5 size-1 shrink-0 rounded-full bg-fg-subtle" />
                      {line}
                    </li>
                  ))}
              </ul>
            </li>
          ))}
        </ol>
      ) : null}
    </GlassPanel>
  );
}
