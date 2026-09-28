import { openUrl } from "@tauri-apps/plugin-opener";
import { Check, CloudDownload, ExternalLink, Heart, Library } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { GlassCard } from "@/components/ui/glass";
import { notify } from "@/components/ui/toast";
import { formatNumber } from "@/lib/format";
import type { MarketItem } from "../api";

interface MarketCardProps {
  item: MarketItem;
  installing: boolean;
  onInstall: () => void;
  onOpenLibrary: () => void;
}

export function MarketCard({ item, installing, onInstall, onOpenLibrary }: MarketCardProps) {
  const { t } = useTranslation("market");
  const pageUrl = item.pageUrl;
  const missing = item.bodies.filter((b) => !item.installedBodies.includes(b));
  const partial = item.installedBodies.length > 0 && missing.length > 0;
  const openPage = pageUrl ? () => void openUrl(pageUrl).catch(notify.error) : undefined;

  return (
    <GlassCard className="group flex flex-col overflow-hidden">
      <div className="relative aspect-square w-full overflow-hidden bg-white/[0.03]">
        {item.thumbnail ? (
          <img
            src={item.thumbnail}
            alt=""
            loading="lazy"
            // AlphaConsole's image bucket refuses foreign referrers.
            referrerPolicy="no-referrer"
            className="size-full object-cover transition-transform duration-500 group-hover:scale-[1.03]"
          />
        ) : null}
        {item.installed ? (
          <Badge tone="success" dot className="absolute top-2.5 left-2.5">
            {t("card.inLibrary")}
          </Badge>
        ) : null}
      </div>
      <div className="flex flex-1 flex-col gap-2 p-3">
        <div className="min-w-0">
          <p className="truncate text-[13px] font-semibold" title={item.title}>
            {item.title}
          </p>
          <p className="truncate text-xs text-fg-subtle">
            {item.author ?? t("card.unknownAuthor")}
            {item.downloads != null
              ? ` · ${t("card.downloads", { count: item.downloads, value: formatNumber(item.downloads) })}`
              : ""}
            {item.likes != null ? (
              <span className="ml-1.5 inline-flex items-center gap-0.5 align-[-1px]">
                <Heart className="size-3" />
                {formatNumber(item.likes)}
              </span>
            ) : null}
          </p>
        </div>
        {item.description ? <p className="line-clamp-2 text-xs text-fg-muted">{item.description}</p> : null}
        {item.bodies.length ? (
          <div className="flex flex-wrap gap-1">
            {item.bodies.map((b) =>
              item.installedBodies.includes(b) ? (
                <Badge key={b} tone="success" title={t("card.bodyInstalled")}>
                  <Check className="size-3" />
                  {b}
                </Badge>
              ) : (
                <Badge key={b}>{b}</Badge>
              ),
            )}
          </div>
        ) : null}
        <div className="mt-auto flex gap-1.5 pt-1">
          {item.installed ? (
            <Button size="sm" variant="secondary" className="flex-1" onClick={onOpenLibrary}>
              <Library />
              {t("card.showInLibrary")}
            </Button>
          ) : (
            <Button size="sm" variant="primary" className="flex-1" onClick={onInstall} loading={installing}>
              <CloudDownload />
              <span className="truncate">
                {partial ? t("card.addMissing", { bodies: missing.join(", ") }) : t("card.install")}
              </span>
            </Button>
          )}
          {openPage ? (
            <Button size="icon-sm" variant="ghost" onClick={openPage} aria-label={t("card.openPage")}>
              <ExternalLink />
            </Button>
          ) : null}
        </div>
      </div>
    </GlassCard>
  );
}
