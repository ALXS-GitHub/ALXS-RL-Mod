import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { notify } from "@/components/ui/toast";
import { isTauri } from "@/lib/ipc";
import { type CatalogItem, itemsApi, type SwapRequest } from "./api";

export const itemsKeys = {
  catalog: ["catalog"] as const,
  swaps: ["swaps"] as const,
  history: ["swaps", "history"] as const,
  integrity: ["integrity"] as const,
  thumbnail: (pkg: string) => ["thumbnail", pkg] as const,
  paints: ["paints"] as const,
  itemPaints: (id: number) => ["paints", "item", id] as const,
};

/** The game's paints (read once per app run). */
export function usePaints() {
  return useQuery({
    queryKey: itemsKeys.paints,
    queryFn: itemsApi.paints,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

/** Paints a catalog item accepts (empty: not paintable). */
export function useItemPaints(itemId: number | null) {
  return useQuery({
    queryKey: itemsKeys.itemPaints(itemId ?? 0),
    queryFn: () => itemsApi.itemPaints(itemId ?? 0),
    enabled: itemId !== null,
    staleTime: 10 * 60_000,
  });
}

/** Localised paint name: known paints are translated, "… Glow" too. */
export function usePaintLabel() {
  const { t } = useTranslation("items");
  return useCallback(
    (label: string) => {
      const glow = label.endsWith(" Glow");
      const base = glow ? label.slice(0, -" Glow".length) : label;
      const key = base.toLowerCase().replace(/ (\w)/g, (_, c: string) => c.toUpperCase());
      const name = t(`paint.${key}`, { defaultValue: base });
      return glow ? t("paint.glow", { name }) : name;
    },
    [t],
  );
}

export function useCatalog(enabled = true) {
  return useQuery({ queryKey: itemsKeys.catalog, queryFn: itemsApi.catalog, staleTime: 5 * 60_000, enabled });
}

export function useRefreshCatalog() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: itemsApi.refreshCatalog,
    onSuccess: (snap) => qc.setQueryData(itemsKeys.catalog, snap),
    onError: notify.error,
  });
}

export function useSwaps() {
  return useQuery({ queryKey: itemsKeys.swaps, queryFn: itemsApi.swaps });
}

export function useSwapHistory(enabled: boolean) {
  return useQuery({ queryKey: itemsKeys.history, queryFn: itemsApi.history, enabled });
}

function useInvalidateSwaps() {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: itemsKeys.swaps });
    void qc.invalidateQueries({ queryKey: itemsKeys.integrity });
  };
}

export function useApplySwap() {
  const { t } = useTranslation("items");
  const invalidate = useInvalidateSwaps();
  return useMutation({
    mutationFn: (req: SwapRequest) => itemsApi.apply(req),
    onSuccess: (swap) => {
      invalidate();
      notify.success(t("toast.applied"), `${swap.ownedLabel} → ${swap.wantedLabel}`);
    },
    onError: notify.error,
  });
}

export function useRestoreSwap() {
  const { t } = useTranslation("items");
  const invalidate = useInvalidateSwaps();
  return useMutation({
    mutationFn: (id: string) => itemsApi.restore(id),
    onSuccess: () => {
      invalidate();
      notify.success(t("toast.restored"));
    },
    onError: notify.error,
  });
}

export function useIntegrityCheck() {
  return useQuery({ queryKey: itemsKeys.integrity, queryFn: itemsApi.integrityCheck, staleTime: 60_000 });
}

export function useIntegrityReapply() {
  const qc = useQueryClient();
  const { t } = useTranslation("items");
  return useMutation({
    mutationFn: itemsApi.integrityReapply,
    onSuccess: (report) => {
      qc.setQueryData(itemsKeys.integrity, { ...report, pending: [], gameUpdated: false, filesReverted: 0 });
      void qc.invalidateQueries({ queryKey: itemsKeys.swaps });
      if (report.failures.length === 0) notify.success(t("integrity.done"));
    },
    onError: notify.error,
  });
}

/**
 * Lazy thumbnail for a catalog item: resolved once per package (the engine
 * extracts and caches the PNG), then served through the asset protocol.
 */
export function useThumbnail(item: Pick<CatalogItem, "thumbnailPackage"> | undefined) {
  const pkg = item?.thumbnailPackage ?? null;
  const q = useQuery({
    queryKey: itemsKeys.thumbnail(pkg ?? ""),
    queryFn: () => itemsApi.thumbnail(pkg as string),
    enabled: Boolean(pkg) && isTauri,
    staleTime: Number.POSITIVE_INFINITY,
    gcTime: 10 * 60_000,
    retry: false,
  });
  return q.data ? convertFileSrc(q.data) : null;
}
