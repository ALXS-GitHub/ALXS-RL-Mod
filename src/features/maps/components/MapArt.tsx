import { convertFileSrc } from "@tauri-apps/api/core";
import { useState } from "react";
import { cn } from "@/lib/cn";
import { isTauri } from "@/lib/ipc";

function hash(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

/** Local preview file → asset URL (served from the app data scope). */
export function localPreview(folder: string, file: string | null): string | null {
  if (!file || !isTauri) return null;
  const sep = folder.includes("\\") ? "\\" : "/";
  return convertFileSrc(`${folder}${sep}${file}`);
}

interface MapArtProps {
  name: string;
  src: string | null;
  className?: string;
}

/**
 * Map cover: the real preview when there is one, otherwise a deterministic
 * neutral tile with the name's initials (faint hue derived from the name).
 */
export function MapArt({ name, src, className }: MapArtProps) {
  const [failed, setFailed] = useState(false);
  const h = hash(name);
  const hueA = h % 360;
  const initials = name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase())
    .join("");

  return (
    <div className={cn("relative overflow-hidden bg-bg-elevated", className)}>
      {src && !failed ? (
        <>
          <img
            src={src}
            alt=""
            loading="lazy"
            draggable={false}
            onError={() => setFailed(true)}
            className="absolute inset-0 h-full w-full object-cover"
          />
          {/* Legibility scrim for the title drawn over a real screenshot. */}
          <div className="absolute inset-0 bg-[linear-gradient(180deg,transparent_50%,rgb(8_9_13/0.8))]" />
        </>
      ) : (
        <div className="absolute inset-0 grid place-items-center bg-white/[0.03]">
          {/* Neutral placeholder: initials with a faint hue derived from the name. */}
          <span
            className="-mt-6 text-2xl font-semibold tracking-tight"
            style={{ color: `oklch(0.75 0.06 ${hueA} / 0.35)` }}
          >
            {initials}
          </span>
        </div>
      )}
    </div>
  );
}
