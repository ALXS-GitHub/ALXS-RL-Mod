import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink, Search, Star, UserRound, X } from "lucide-react";
import { motion } from "motion/react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ErrorState, Skeleton } from "@/components/ui/feedback";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { notify } from "@/components/ui/toast";
import { Tooltip } from "@/components/ui/tooltip";
import { cn } from "@/lib/cn";
import { type AppConfig, useConfig, useUpdateConfig } from "@/lib/game";
import { PLATFORMS, type Platform } from "../api";
import { useMmrLookup, useTrackerSession } from "../queries";

interface Target {
  platform: Platform;
  player: string;
}

type TrackerConfig = AppConfig["tracker"];

function isPlatform(v: string | null | undefined): v is Platform {
  return Boolean(v && (PLATFORMS as readonly string[]).includes(v));
}

const same = (a: Target | null, platform: string | null | undefined, player: string | null | undefined) =>
  Boolean(
    a && platform && player && a.platform === platform && a.player.toLowerCase() === player.toLowerCase(),
  );

/** A clickable player chip (own profile or favorite). */
function PlayerChip({
  label,
  platform,
  active,
  icon,
  onSelect,
  onRemove,
  removeLabel,
}: {
  label: string;
  platform: string;
  active: boolean;
  icon: React.ReactNode;
  onSelect: () => void;
  onRemove?: () => void;
  removeLabel?: string;
}) {
  const { t } = useTranslation("tracker");
  return (
    <span
      className={cn(
        "group inline-flex h-7 items-center gap-1.5 rounded-[6px] border pr-1 pl-2 text-xs transition-colors",
        active
          ? "border-[var(--color-accent)] bg-[color-mix(in_oklab,var(--color-accent)_10%,transparent)] text-fg"
          : "border-line text-fg-muted hover:border-line-strong hover:text-fg",
      )}
    >
      <button type="button" onClick={onSelect} className="flex items-center gap-1.5">
        {icon}
        <span className="max-w-40 truncate font-medium">{label}</span>
        <span className="text-fg-subtle">{t(`mmr.platforms.${platform}`)}</span>
      </button>
      {onRemove ? (
        <button
          type="button"
          onClick={onRemove}
          aria-label={removeLabel}
          className="grid size-5 place-items-center rounded-[4px] text-fg-subtle opacity-0 group-hover:opacity-100 hover:bg-white/[0.08] hover:text-fg"
        >
          <X className="size-3" />
        </button>
      ) : (
        <span className="w-1" />
      )}
    </span>
  );
}

/** The "MMR" tab: tracker.gg lookup, your profile and favorite players. */
export function MmrView() {
  const { t } = useTranslation("tracker");
  const { data: config } = useConfig();
  const updateConfig = useUpdateConfig();
  const { data: session } = useTrackerSession();
  const [platform, setPlatform] = useState<Platform>("epic");
  const [player, setPlayer] = useState("");
  const [submitted, setSubmitted] = useState<Target | null>(null);
  const tracker = config?.tracker;
  const favorites = tracker?.favorites ?? [];

  const show = useCallback((target: Target) => {
    setPlatform(target.platform);
    setPlayer(target.player);
    setSubmitted(target);
  }, []);

  // Open on your own profile, once (later config edits must not reset the view).
  const started = useRef(false);
  useEffect(() => {
    if (started.current || !tracker) return;
    started.current = true;
    if (isPlatform(tracker.platform) && tracker.playerId)
      show({ platform: tracker.platform, player: tracker.playerId });
  }, [tracker, show]);

  const lookup = useMmrLookup(submitted?.platform ?? null, submitted?.player ?? null);

  const saveTracker = (next: Partial<TrackerConfig>) => {
    if (!tracker) return;
    updateConfig.mutate({ tracker: { ...tracker, ...next } });
  };

  const submit = () => {
    const p = player.trim();
    if (!p) return;
    setSubmitted({ platform, player: p });
    // The first lookup becomes your profile; later ones are just lookups.
    if (!tracker?.playerId) saveTracker({ platform, playerId: p });
  };

  const isMine = same(submitted, tracker?.platform, tracker?.playerId);
  const favorite = favorites.find((f) => same(submitted, f.platform, f.playerId));
  const toggleFavorite = () => {
    if (!submitted) return;
    saveTracker({
      favorites: favorite
        ? favorites.filter((f) => f !== favorite)
        : [...favorites, { platform: submitted.platform, playerId: submitted.player }],
    });
  };

  const deltas = new Map((session?.mmr ?? []).map((l) => [l.playlist, l.latest - l.baseline]));
  const profileUrl = submitted
    ? `https://rocketleague.tracker.network/rocket-league/profile/${submitted.platform}/${encodeURIComponent(submitted.player)}/overview`
    : null;

  if (tracker && !tracker.onlineRatings) {
    return (
      <section className="flex max-w-2xl flex-col gap-3 rounded-lg border border-line p-4">
        <p className="text-sm font-semibold">{t("mmr.off.title")}</p>
        <p className="text-[13px] text-fg-muted">{t("mmr.off.body")}</p>
        <Button
          variant="primary"
          className="self-start"
          loading={updateConfig.isPending}
          onClick={() => saveTracker({ onlineRatings: true })}
        >
          {t("mmr.off.enable")}
        </Button>
      </section>
    );
  }

  return (
    <section className="flex flex-col gap-3">
      <form
        className="flex max-w-2xl items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <Select
          className="w-32 shrink-0"
          value={platform}
          onValueChange={setPlatform}
          options={PLATFORMS.map((p) => ({ value: p, label: t(`mmr.platforms.${p}`) }))}
        />
        <Input
          value={player}
          onChange={(e) => setPlayer(e.target.value)}
          placeholder={t("mmr.placeholder")}
        />
        <Button
          type="submit"
          variant="primary"
          size="icon"
          className="shrink-0"
          aria-label={t("mmr.lookup")}
          loading={lookup.isFetching}
        >
          {lookup.isFetching ? null : <Search />}
        </Button>
        {submitted && !isMine ? (
          <Tooltip content={favorite ? t("mmr.unfavorite") : t("mmr.favorite")}>
            <Button
              type="button"
              size="icon"
              variant="ghost"
              className={cn("shrink-0", favorite && "text-warning")}
              aria-label={favorite ? t("mmr.unfavorite") : t("mmr.favorite")}
              aria-pressed={Boolean(favorite)}
              onClick={toggleFavorite}
            >
              <Star className={cn(favorite && "fill-current")} />
            </Button>
          </Tooltip>
        ) : null}
      </form>

      {/* One click to look up yourself or a favorite. */}
      {tracker?.playerId || favorites.length > 0 ? (
        <div className="flex flex-wrap items-center gap-1.5">
          {isPlatform(tracker?.platform) && tracker?.playerId ? (
            <PlayerChip
              label={tracker.playerId}
              platform={tracker.platform}
              active={isMine}
              icon={<UserRound className="size-3.5" />}
              onSelect={() =>
                isPlatform(tracker.platform) &&
                tracker.playerId &&
                show({ platform: tracker.platform, player: tracker.playerId })
              }
            />
          ) : null}
          {favorites.map((f) =>
            isPlatform(f.platform) ? (
              <PlayerChip
                key={`${f.platform}:${f.playerId}`}
                label={f.playerId}
                platform={f.platform}
                active={same(submitted, f.platform, f.playerId)}
                icon={<Star className="size-3.5 fill-current text-warning" />}
                onSelect={() => isPlatform(f.platform) && show({ platform: f.platform, player: f.playerId })}
                onRemove={() => saveTracker({ favorites: favorites.filter((x) => x !== f) })}
                removeLabel={t("mmr.unfavorite")}
              />
            ) : null,
          )}
        </div>
      ) : null}

      <div className="flex flex-wrap items-center gap-2 text-[11px] text-fg-subtle">
        <span>{t("mmr.hint")}</span>
        {submitted && !isMine ? (
          <button
            type="button"
            className="text-fg-muted underline-offset-2 hover:text-fg hover:underline"
            onClick={() => saveTracker({ platform: submitted.platform, playerId: submitted.player })}
          >
            {t("mmr.setMine")}
          </button>
        ) : null}
        {lookup.data?.displayName ? <Badge tone="accent">{lookup.data.displayName}</Badge> : null}
      </div>

      {lookup.isError ? (
        <div className="flex max-w-xl flex-col gap-2">
          <ErrorState error={lookup.error} onRetry={() => void lookup.refetch()} />
          {profileUrl ? (
            <Button size="sm" variant="ghost" onClick={() => void openUrl(profileUrl).catch(notify.error)}>
              <ExternalLink />
              {t("mmr.openTracker")}
            </Button>
          ) : null}
        </div>
      ) : lookup.isPending && submitted ? (
        <Skeleton className="h-40" />
      ) : lookup.data ? (
        <ul className="mt-1 grid gap-2 @2xl:grid-cols-2 @5xl:grid-cols-3">
          {lookup.data.playlists
            .filter((p) => p.rating !== null)
            .map((p, i) => {
              // Session deltas only make sense for your own profile.
              const delta = isMine ? deltas.get(p.playlist) : undefined;
              return (
                <motion.li
                  key={p.playlist}
                  initial={{ opacity: 0, x: -8 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: i * 0.03 }}
                  className="flex items-center gap-3 rounded-[8px] border border-line bg-white/[0.02] px-3 py-2.5"
                >
                  {p.tierIcon ? (
                    <img src={p.tierIcon} alt="" className="size-9 shrink-0 object-contain" loading="lazy" />
                  ) : (
                    <div className="size-9 shrink-0 rounded-[7px] bg-white/[0.04]" />
                  )}
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-[13px] font-medium">{p.playlist}</p>
                    <p className="truncate text-xs text-fg-subtle">
                      {[p.tier, p.division].filter(Boolean).join(" · ") || t("mmr.unranked")}
                    </p>
                  </div>
                  <div className="text-right">
                    <p className="text-[15px] font-semibold tabular-nums">{p.rating}</p>
                    {delta ? (
                      <p className={`text-xs tabular-nums ${delta > 0 ? "text-success" : "text-danger"}`}>
                        {delta > 0 ? "+" : ""}
                        {delta} {t("mmr.session")}
                      </p>
                    ) : null}
                  </div>
                </motion.li>
              );
            })}
        </ul>
      ) : (
        <p className="text-[13px] text-fg-muted">{t("mmr.empty")}</p>
      )}
    </section>
  );
}
