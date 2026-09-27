import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ballKeys } from "@/features/ball/queries";
import { decalKeys } from "@/features/decals/queries";
import { type MarketKind, type MarketSort, type MarketSource, marketApi } from "./api";

export const marketKeys = {
  all: ["market"] as const,
  browse: (source: MarketSource, kind: MarketKind, query: string, sort: MarketSort) =>
    ["market", "browse", source, kind, query, sort] as const,
};

export function useMarketBrowse(source: MarketSource, kind: MarketKind, query: string, sort: MarketSort) {
  return useQuery({
    queryKey: marketKeys.browse(source, kind, query, sort),
    queryFn: () => marketApi.browse(source, kind, query, sort),
    placeholderData: keepPreviousData,
    staleTime: 5 * 60_000,
  });
}

/** Installs a pack; the libraries and the "installed" flags refresh. */
export function useMarketInstall() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: marketApi.install,
    onSuccess: (_report, { item }) => {
      void qc.invalidateQueries({ queryKey: marketKeys.all });
      void qc.invalidateQueries({ queryKey: item.kind === "decal" ? decalKeys.library : ballKeys.library });
    },
  });
}
