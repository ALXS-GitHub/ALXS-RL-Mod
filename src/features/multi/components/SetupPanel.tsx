import { openUrl } from "@tauri-apps/plugin-opener";
import { Check, CircleAlert, CircleDashed, ExternalLink, RefreshCw, Users } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ErrorState, Skeleton } from "@/components/ui/feedback";
import { GlassPanel } from "@/components/ui/glass";
import { notify } from "@/components/ui/toast";
import type { MultiStatus } from "../api";
import { useBakkesSetup, useBakkesUndo, useMultiStatus } from "../queries";

export const TAILSCALE_DOWNLOAD = "https://tailscale.com/download";

type RowState = "ok" | "warn" | "todo";

function Row({
  state,
  label,
  detail,
  action,
}: {
  state: RowState;
  label: string;
  detail?: ReactNode;
  action?: ReactNode;
}) {
  const Icon = state === "ok" ? Check : state === "warn" ? CircleAlert : CircleDashed;
  const color = state === "ok" ? "text-success" : state === "warn" ? "text-warning" : "text-fg-subtle";
  return (
    <div className="flex items-start justify-between gap-3 border-line border-b py-2.5 last:border-b-0">
      <div className="flex min-w-0 items-start gap-2.5">
        <Icon className={`mt-0.5 size-3.5 shrink-0 ${color}`} />
        <div className="min-w-0">
          <p className="text-[13px] font-medium">{label}</p>
          {detail ? <p className="mt-0.5 text-xs text-fg-muted">{detail}</p> : null}
        </div>
      </div>
      {action ? <div className="shrink-0">{action}</div> : null}
    </div>
  );
}

/** Everything that must be in place before hosting or joining. */
export function isReady(s: MultiStatus): boolean {
  return (
    s.tailscale.state === "running" &&
    Boolean(s.bakkes.dataDir) &&
    s.bakkes.rocketPluginInstalled &&
    !(s.gameRunning && s.gameWithEac)
  );
}

export function SetupPanel() {
  const { t } = useTranslation("multi");
  const status = useMultiStatus();
  const setup = useBakkesSetup();
  const undo = useBakkesUndo();
  const s = status.data;

  return (
    <GlassPanel className="flex flex-col gap-2 p-4">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold">{t("setup.title")}</h2>
        <Button
          size="icon-sm"
          variant="ghost"
          aria-label={t("setup.refresh")}
          title={t("setup.refresh")}
          onClick={() => void status.refetch()}
        >
          <RefreshCw className={status.isFetching ? "animate-spin" : undefined} />
        </Button>
      </div>
      {status.isError ? (
        <ErrorState error={status.error} onRetry={() => void status.refetch()} />
      ) : !s ? (
        <Skeleton className="h-48" />
      ) : (
        <div className="flex flex-col">
          <Row
            state={
              s.tailscale.state === "running" ? "ok" : s.tailscale.state === "notInstalled" ? "todo" : "warn"
            }
            label="Tailscale"
            detail={
              s.tailscale.state === "running"
                ? t("setup.tailscale.running", {
                    ip: s.tailscale.selfIpv4 ?? "?",
                    name: s.tailscale.selfName ?? "?",
                  })
                : t(`setup.tailscale.${s.tailscale.state}`)
            }
            action={
              s.tailscale.state === "notInstalled" ? (
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => void openUrl(TAILSCALE_DOWNLOAD).catch(notify.error)}
                >
                  <ExternalLink />
                  {t("setup.tailscale.download")}
                </Button>
              ) : null
            }
          />
          <Row
            state={s.bakkes.dataDir ? "ok" : "todo"}
            label="BakkesMod"
            detail={
              !s.bakkes.dataDir
                ? t("setup.bakkes.missing")
                : s.bakkes.injectorPath
                  ? s.bakkes.injectorRunning
                    ? t("setup.bakkes.running")
                    : t("setup.bakkes.ready")
                  : t("setup.bakkes.noInjector")
            }
          />
          <Row
            state={s.bakkes.rocketPluginInstalled ? (s.bakkes.rocketPluginLoaded ? "ok" : "warn") : "todo"}
            label="Rocket Plugin"
            detail={
              !s.bakkes.rocketPluginInstalled
                ? t("setup.rocketPlugin.missing")
                : s.bakkes.rocketPluginLoaded
                  ? t("setup.rocketPlugin.loaded")
                  : t("setup.rocketPlugin.notLoaded")
            }
          />
          <Row
            state={s.bakkes.rconReady ? "ok" : "todo"}
            label={t("setup.rcon.label")}
            detail={
              s.bakkes.rconReady ? t("setup.rcon.ready", { port: s.bakkes.rconPort }) : t("setup.rcon.off")
            }
            action={
              s.bakkes.dataDir ? (
                s.bakkes.rconReady ? (
                  <Button size="sm" variant="ghost" loading={undo.isPending} onClick={() => undo.mutate()}>
                    {t("setup.rcon.undo")}
                  </Button>
                ) : (
                  <Button
                    size="sm"
                    variant="secondary"
                    loading={setup.isPending}
                    onClick={() =>
                      setup.mutate(undefined, {
                        onSuccess: () =>
                          notify.success(
                            t("setup.rcon.done"),
                            s.gameRunning ? t("setup.rcon.restart") : undefined,
                          ),
                      })
                    }
                  >
                    {t("setup.rcon.enable")}
                  </Button>
                )
              ) : null
            }
          />
          <Row
            state={s.gameRunning ? (s.gameWithEac ? "warn" : "ok") : "ok"}
            label={t("setup.game.label")}
            detail={
              s.gameRunning
                ? s.gameWithEac
                  ? t("setup.game.eac")
                  : t("setup.game.noEac")
                : t("setup.game.closed")
            }
          />
        </div>
      )}

      {s && s.tailscale.state === "running" ? (
        <div className="mt-2 flex flex-col gap-2">
          <p className="flex items-center gap-1.5 text-xs text-fg-subtle">
            <Users className="size-3.5" />
            {t("setup.peers", { count: s.tailscale.peers.length })}
          </p>
          {s.tailscale.peers.length ? (
            <div className="flex flex-wrap gap-1.5">
              {s.tailscale.peers.map((p) => (
                <Badge
                  key={`${p.name}-${p.ipv4}`}
                  tone={p.online ? "success" : "neutral"}
                  dot
                  title={p.ipv4 ?? undefined}
                >
                  {p.name}
                </Badge>
              ))}
            </div>
          ) : (
            <p className="text-xs text-fg-muted">{t("setup.noPeers")}</p>
          )}
        </div>
      ) : null}
    </GlassPanel>
  );
}
