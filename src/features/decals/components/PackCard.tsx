import { convertFileSrc } from "@tauri-apps/api/core";
import { ImageOff, Layers } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { GlassCard } from "@/components/ui/glass";
import { cn } from "@/lib/cn";
import type { DecalPack } from "../api";

export const assetUrl = (path: string | null) => (path ? convertFileSrc(path) : null);

export function PackPreview({
  src,
  className,
  alt,
}: {
  src: string | null;
  className?: string;
  alt: string;
}) {
  return (
    <div
      className={cn(
        "relative overflow-hidden bg-[repeating-conic-gradient(rgb(255_255_255/0.04)_0%_25%,transparent_0%_50%)] bg-[length:16px_16px]",
        className,
      )}
    >
      {src ? (
        <img src={src} alt={alt} loading="lazy" draggable={false} className="h-full w-full object-cover" />
      ) : (
        <div className="grid h-full w-full place-items-center text-fg-subtle">
          <ImageOff className="size-5" />
        </div>
      )}
    </div>
  );
}

export function PackCard({
  pack,
  selected,
  active,
  onSelect,
}: {
  pack: DecalPack;
  selected: boolean;
  active: boolean;
  onSelect: () => void;
}) {
  const { t } = useTranslation("decals");
  return (
    <GlassCard
      selected={selected}
      role="button"
      tabIndex={0}
      onClick={onSelect}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") onSelect();
      }}
      className={cn("group cursor-pointer overflow-hidden", !pack.supported && "opacity-70")}
    >
      <div className="relative">
        <PackPreview src={assetUrl(pack.previewPath)} alt={pack.displayName} className="aspect-[4/3]" />
        {pack.maskPreviewPath ? (
          <div className="absolute right-2 bottom-2 size-10 overflow-hidden rounded-[6px] border border-white/15">
            <PackPreview
              src={assetUrl(pack.maskPreviewPath)}
              alt={t("pack.mask")}
              className="h-full w-full"
            />
          </div>
        ) : null}
        <div className="absolute top-2 left-2 flex gap-1.5">
          {active ? (
            <Badge tone="success" dot>
              {t("pack.inGame")}
            </Badge>
          ) : null}
          {pack.hybrid ? <Badge tone="warning">{t("pack.hybrid")}</Badge> : null}
          {pack.universal ? <Badge tone="warning">{t("pack.universal")}</Badge> : null}
          {pack.supported ? (
            <Badge tone="accent">{pack.bodyName ?? pack.bodyFolder}</Badge>
          ) : (
            <Badge tone="neutral">{t(pack.maskPath ? "pack.bodyUnsupported" : "pack.noMask")}</Badge>
          )}
        </div>
      </div>
      <div className="p-3">
        <p className="truncate text-sm font-semibold">{pack.displayName}</p>
        <p className="mt-0.5 flex items-center gap-1.5 truncate text-[11px] text-fg-subtle">
          <Layers className="size-3 shrink-0" />
          {pack.packName} · {pack.bodyFolder}
        </p>
      </div>
    </GlassCard>
  );
}
