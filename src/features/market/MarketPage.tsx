import { useNavigate } from "@tanstack/react-router";
import { Globe, PackageSearch, Store } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { EmptyState, ErrorState, Skeleton } from "@/components/ui/feedback";
import { SearchInput } from "@/components/ui/input";
import { Page, PageHeader } from "@/components/ui/layout";
import { Segmented } from "@/components/ui/segmented";
import { Select } from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { notify } from "@/components/ui/toast";
import { BrowseTab } from "@/features/maps/components/BrowseTab";
import { MARKET_SORTS, type MarketItem, type MarketKind, type MarketSort, type MarketSource } from "./api";
import { InstallDialog } from "./components/InstallDialog";
import { MarketCard } from "./components/MarketCard";
import { useMarketBrowse, useMarketInstall } from "./queries";

type Tab = "maps" | MarketKind;

function useDebounced<T>(value: T, delay = 350): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setV(value), delay);
    return () => clearTimeout(id);
  }, [value, delay]);
  return v;
}

function PacksTab({ kind }: { kind: MarketKind }) {
  const { t } = useTranslation("market");
  const navigate = useNavigate();
  const [source, setSource] = useState<MarketSource>("alphaConsole");
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<MarketSort>("downloads");
  const debounced = useDebounced(query);
  const browse = useMarketBrowse(source, kind, debounced, sort);
  const install = useMarketInstall();
  const [asking, setAsking] = useState<MarketItem | null>(null);

  const libraryPath = kind === "decal" ? "/decals" : "/ball";
  const run = (item: MarketItem, convert: boolean) =>
    install.mutate(
      { item, convert },
      {
        onSuccess: (r) => {
          setAsking(null);
          notify.success(
            t("done.title", { name: r.pack }),
            [
              t("done.variants", { count: r.variants }),
              r.converted ? t("done.converted", { count: r.converted }) : null,
              r.skipped ? t("done.skipped", { count: r.skipped }) : null,
              r.failed.length ? t("done.failed", { count: r.failed.length }) : null,
            ]
              .filter(Boolean)
              .join(" · "),
          );
        },
        onError: notify.error,
      },
    );
  // AlphaConsole decal packs may need converting: ask every time.
  const onInstall = (item: MarketItem) =>
    item.kind === "decal" && !item.ready ? setAsking(item) : run(item, false);

  const data = browse.data;
  const sortOptions = MARKET_SORTS.map((value) => ({ value, label: t(`sort.${value}`) }));

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <Segmented
          value={source}
          onValueChange={setSource}
          size="sm"
          options={[
            { value: "alphaConsole", label: "AlphaConsole", icon: <Globe /> },
            { value: "rlDesigner", label: "RL-Designer", icon: <Globe /> },
          ]}
        />
        <SearchInput
          value={query}
          onValueChange={setQuery}
          placeholder={t("search")}
          className="w-full max-w-sm"
        />
        {source === "alphaConsole" ? (
          <Select value={sort} onValueChange={setSort} options={sortOptions} className="w-44" />
        ) : null}
        {data?.total != null ? (
          <span className="ml-auto text-xs text-fg-subtle tabular-nums">
            {data.truncated
              ? t("countTruncated", { shown: data.items.length, total: data.total })
              : t("count", { count: data.total })}
          </span>
        ) : null}
      </div>

      {browse.error ? (
        <ErrorState error={browse.error} onRetry={() => void browse.refetch()} />
      ) : !data ? (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3">
          {Array.from({ length: 10 }, (_, i) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: static placeholders
            <Skeleton key={i} className="aspect-[3/4] rounded-xl" />
          ))}
        </div>
      ) : data.items.length === 0 ? (
        <EmptyState icon={PackageSearch} title={t("empty.title")} description={t("empty.description")} />
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3">
          {data.items.map((item) => (
            <MarketCard
              key={`${item.source}:${item.id}`}
              item={item}
              installing={install.isPending && install.variables?.item.id === item.id}
              onInstall={() => onInstall(item)}
              onOpenLibrary={() => void navigate({ to: libraryPath })}
            />
          ))}
        </div>
      )}
      {data?.truncated ? <p className="text-center text-xs text-fg-subtle">{t("truncatedHint")}</p> : null}
      <p className="text-[11px] text-fg-subtle">{t(`credits.${source}`)}</p>

      <InstallDialog
        item={asking}
        busy={install.isPending}
        onCancel={() => setAsking(null)}
        onConfirm={(convert) => asking && run(asking, convert)}
      />
    </div>
  );
}

export function MarketPage() {
  const { t } = useTranslation("market");
  const navigate = useNavigate();
  const [tab, setTab] = useState<Tab>("maps");

  return (
    <Page>
      <PageHeader icon={Store} eyebrow={t("eyebrow")} title={t("title")} subtitle={t("subtitle")} />
      <Tabs value={tab} onValueChange={(v) => setTab(v as Tab)}>
        <TabsList>
          <TabsTrigger value="maps">{t("tabs.maps")}</TabsTrigger>
          <TabsTrigger value="decal">{t("tabs.decal")}</TabsTrigger>
          <TabsTrigger value="ball">{t("tabs.ball")}</TabsTrigger>
        </TabsList>
        <TabsContent value="maps">
          <BrowseTab onShowLibrary={() => void navigate({ to: "/maps" })} />
        </TabsContent>
        <TabsContent value="decal">
          <PacksTab kind="decal" />
        </TabsContent>
        <TabsContent value="ball">
          <PacksTab kind="ball" />
        </TabsContent>
      </Tabs>
    </Page>
  );
}
