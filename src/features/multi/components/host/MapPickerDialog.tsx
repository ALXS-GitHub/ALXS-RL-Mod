import { Check, Star } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { SearchInput } from "@/components/ui/input";
import type { MapEntry } from "@/features/maps/api";
import { localPreview, MapArt } from "@/features/maps/components/MapArt";
import { cn } from "@/lib/cn";

/** Library maps as a grid of greyed thumbnails: click one to pick it. */
export function MapPickerDialog({
  open,
  onOpenChange,
  maps,
  selectedId,
  onPick,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  maps: MapEntry[];
  selectedId: string | null;
  onPick: (map: MapEntry) => void;
}) {
  const { t } = useTranslation("multi");
  const [query, setQuery] = useState("");
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return q
      ? maps.filter((m) => m.name.toLowerCase().includes(q) || m.author?.toLowerCase().includes(q))
      : maps;
  }, [maps, query]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg" title={t("host.picker.title")} description={t("host.picker.description")}>
        <div className="flex flex-col gap-3">
          <SearchInput value={query} onValueChange={setQuery} placeholder={t("host.picker.search")} />
          {shown.length ? (
            <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-3">
              {shown.map((m) => {
                const selected = m.id === selectedId;
                return (
                  <button
                    key={m.id}
                    type="button"
                    onClick={() => {
                      onPick(m);
                      onOpenChange(false);
                    }}
                    className={cn(
                      "group relative overflow-hidden rounded-[8px] border text-left outline-none transition-colors",
                      selected
                        ? "border-[var(--color-accent)]"
                        : "border-line hover:border-line-strong focus-visible:border-line-strong",
                    )}
                  >
                    <MapArt
                      name={m.name}
                      src={localPreview(m.folder, m.previewFile) ?? m.previewUrl}
                      className={cn(
                        "aspect-[16/9] w-full transition-[filter] duration-200",
                        selected
                          ? "grayscale-0"
                          : "grayscale group-hover:grayscale-0 group-focus-visible:grayscale-0",
                      )}
                    />
                    <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/80 to-transparent px-2.5 pt-6 pb-2">
                      <p className="flex items-center gap-1 truncate text-xs font-medium text-white">
                        {m.favorite ? <Star className="size-3 shrink-0 fill-current text-warning" /> : null}
                        <span className="truncate">{m.name}</span>
                      </p>
                      {m.author ? <p className="truncate text-[11px] text-white/60">{m.author}</p> : null}
                    </div>
                    {selected ? (
                      <span className="absolute top-2 right-2 grid size-5 place-items-center rounded-full bg-[var(--color-accent)] text-white">
                        <Check className="size-3" />
                      </span>
                    ) : null}
                  </button>
                );
              })}
            </div>
          ) : (
            <p className="py-6 text-center text-xs text-fg-muted">{t("host.picker.empty")}</p>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
