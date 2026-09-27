import { useState } from "react";
import { cn } from "@/lib/cn";
import type { CatalogItem } from "../api";
import { SLOT_ICONS } from "../constants";
import { useThumbnail } from "../queries";

interface ItemThumbProps {
  item: CatalogItem | undefined;
  className?: string;
  /** Icon size fallback when no thumbnail exists. */
  iconClassName?: string;
}

/** Item picture with a neutral slot-icon fallback. */
export function ItemThumb({ item, className, iconClassName }: ItemThumbProps) {
  const src = useThumbnail(item);
  const [loaded, setLoaded] = useState(false);
  const [failed, setFailed] = useState(false);
  const Icon = item ? SLOT_ICONS[item.slot] : SLOT_ICONS.body;

  return (
    <div
      className={cn(
        "relative grid place-items-center overflow-hidden rounded-[7px] bg-white/[0.03]",
        className,
      )}
    >
      {src && !failed ? (
        <img
          src={src}
          alt=""
          draggable={false}
          loading="lazy"
          onLoad={() => setLoaded(true)}
          onError={() => setFailed(true)}
          className={cn(
            "h-full w-full object-contain p-1.5 transition-opacity duration-200",
            loaded ? "opacity-100" : "opacity-0",
          )}
        />
      ) : null}
      {!src || failed || !loaded ? (
        <Icon className={cn("absolute size-5 text-fg-subtle/60", iconClassName)} aria-hidden />
      ) : null}
    </div>
  );
}
