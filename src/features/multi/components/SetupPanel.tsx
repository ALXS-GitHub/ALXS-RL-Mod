import { openUrl } from "@tauri-apps/plugin-opener";
import { Check, CircleAlert, CircleDashed, Download, ExternalLink, RefreshCw, Users } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ErrorState, Skeleton } from "@/components/ui/feedback";
import { GlassPanel } from "@/components/ui/glass";
import { notify } from "@/components/ui/toast";
import { TexturesActions, useTexturesLabel } from "@/features/maps/components/TexturesCard";
import { useTexturesStatus } from "@/features/maps/queries";
import { BAKKESMOD_DOWNLOAD, type MultiStatus, ROCKET_PLUGIN_PAGE, TAILSCALE_DOWNLOAD } from "../api";
import { useBakkesSetup, useBakkesUndo, useInstallRocketPlugin, useMultiStatus } from "../queries";

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
  const installRp = useInstallRocketPlugin();
  const s = status.data;
  // Smart App Control refuses BakkesMod.exe (unsigned) unless it already runs.
  const sacBlocks = !!s && s.smartAppControl === "on" && !s.bakkes.injectorRunning;
  const open = (url: string) => void openUrl(url).catch(notify.error);
  const installRocketPlugin = () =>
    installRp.mutate(undefined, {
      onSuccess: () =>
        notify.info(t("setup.rocketPlugin.installerOpened"), t("setup.rocketPlugin.installerHint")),
      // No BakkesMod installer: its page has the same "Install" button.
      onError: () => open(ROCKET_PLUGIN_PAGE),
    });

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
            state={!s.bakkes.dataDir ? "todo" : sacBlocks ? "warn" : "ok"}
            label="BakkesMod"
            detail={
              !s.bakkes.dataDir
                ? t("setup.bakkes.missing")
                : sacBlocks
                  ? t("setup.bakkes.smartAppControl")
                  : s.bakkes.injectorPath
                    ? s.bakkes.injectorRunning
                      ? t("setup.bakkes.running")
                      : t("setup.bakkes.ready")
                    : t("setup.bakkes.noInjector")
            }
            action={
              s.bakkes.dataDir ? null : (
                <Button size="sm" variant="secondary" onClick={() => open(BAKKESMOD_DOWNLOAD)}>
                  <ExternalLink />
                  {t("setup.tailscale.download")}
                </Button>
              )
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
            action={
              s.bakkes.dataDir && !s.bakkes.rocketPluginInstalled ? (
                <Button
                  size="sm"
                  variant="secondary"
                  loading={installRp.isPending}
                  onClick={installRocketPlugin}
                >
                  <Download />
                  {t("setup.rocketPlugin.install")}
                </Button>
              ) : null
            }
          />
          <Row
            state={s.bakkes.rconReady ? "ok" : "todo"}
            label={t("setup.rcon.label")}
            detail={
              !s.bakkes.rconPluginInstalled
                ? t("setup.rcon.noPlugin")
                : s.bakkes.rconReady
                  ? t("setup.rcon.ready", { port: s.bakkes.rconPort })
                  : t("setup.rcon.off")
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
          <TexturesRow />
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

/** Workshop textures pack: custom maps need it on every player's PC. */
function TexturesRow() {
  const { t } = useTranslation("multi");
  const { data } = useTexturesStatus();
  const label = useTexturesLabel();
  if (!data) return null;
  return (
    <Row
      state={data.installed ? "ok" : "warn"}
      label={t("setup.textures.label")}
      detail={data.installed ? label : `${label} ${t("setup.textures.why")}`}
      action={<TexturesActions compact />}
    />
  );
}
