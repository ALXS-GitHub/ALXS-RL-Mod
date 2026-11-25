import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { Check, CircleAlert, ClipboardPaste, Download, LogIn, Radio } from "lucide-react";
import { type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { GlassCard } from "@/components/ui/glass";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { notify } from "@/components/ui/toast";
import { DEFAULT_TARGET, LABS_TARGETS, targetLabel } from "@/features/maps/api";
import { useDownloadMap, useDownloadProgress, useMapSession, useMaps } from "@/features/maps/queries";
import { toAppError } from "@/lib/errors";
import type { InviteCheck, MultiStatus } from "../api";
import { multiKeys, useHost, useInviteCheck, useJoin } from "../queries";
import { isReady } from "./SetupPanel";

function Card({
  team,
  icon: Icon,
  title,
  description,
  children,
}: {
  team: "orange" | "blue";
  icon: typeof Radio;
  title: string;
  description: string;
  children: ReactNode;
}) {
  const color = team === "orange" ? "var(--color-team-orange)" : "var(--color-team-blue)";
  return (
    <GlassCard className="p-4" style={{ ["--color-accent" as string]: color }}>
      <div className="flex h-full flex-col gap-3">
        <h3 className="flex items-center gap-2 text-sm font-semibold">
          <Icon className="size-4" style={{ color }} />
          {title}
        </h3>
        <p className="text-[13px] text-fg-muted">{description}</p>
        {children}
      </div>
    </GlassCard>
  );
}

/** One status line: green check when fine, amber otherwise. */
function Status({ ok, children }: { ok: boolean; children: ReactNode }) {
  return (
    <p className="flex items-start gap-1.5 text-xs text-fg-muted">
      {ok ? (
        <Check className="mt-0.5 size-3.5 shrink-0 text-success" />
      ) : (
        <CircleAlert className="mt-0.5 size-3.5 shrink-0 text-warning" />
      )}
      <span>{children}</span>
    </p>
  );
}

const ARENA_OPTIONS = LABS_TARGETS.map((t): { value: string; label: string } => ({
  value: t,
  label: targetLabel(t),
}));

export function HostCard({ status, busy }: { status: MultiStatus | undefined; busy: boolean }) {
  const { t } = useTranslation("multi");
  const maps = useMaps();
  const { data: mapSession } = useMapSession();
  const host = useHost();
  const [mapId, setMapId] = useState<string | undefined>(undefined);
  const [arena, setArena] = useState<string | undefined>(undefined);
  const list = maps.data ?? [];
  const selected = list.find((m) => m.id === mapId) ?? list[0];
  const target = arena ?? selected?.preferredTarget ?? DEFAULT_TARGET;
  const alreadyThere = Boolean(
    selected && mapSession && mapSession.mapId === selected.id && mapSession.target === target,
  );
  const blocked = !status ? t("play.checking") : !isReady(status) ? t("play.notReady") : null;

  return (
    <Card team="orange" icon={Radio} title={t("host.title")} description={t("host.description")}>
      {list.length ? (
        <>
          <div className="flex flex-col gap-1.5">
            <span className="text-xs text-fg-subtle">{t("host.map")}</span>
            <Select
              value={selected?.id}
              onValueChange={(id) => {
                setMapId(id);
                setArena(undefined);
              }}
              options={list.map((m) => ({ value: m.id, label: m.name, hint: m.favorite ? "★" : undefined }))}
              placeholder={t("host.pickMap")}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <span className="text-xs text-fg-subtle">{t("host.arena")}</span>
            <Select value={target} onValueChange={setArena} options={ARENA_OPTIONS} />
          </div>
          <Status ok={alreadyThere}>
            {alreadyThere
              ? t("host.alreadyThere", { arena: targetLabel(target) })
              : t("host.willInstall", { arena: targetLabel(target) })}
          </Status>
          <p className="text-xs text-fg-subtle">{t("host.arenaShared")}</p>
        </>
      ) : (
        <p className="text-xs text-fg-muted">{t("host.noMaps")}</p>
      )}
      <div className="mt-auto flex flex-col gap-2">
        <Button
          variant="primary"
          className="w-full"
          disabled={!selected || Boolean(blocked) || busy}
          loading={host.isPending}
          onClick={() => selected && host.mutate({ mapId: selected.id, target })}
        >
          <Radio />
          {t("host.cta")}
        </Button>
        {blocked ? <p className="text-center text-xs text-warning">{blocked}</p> : null}
      </div>
    </Card>
  );
}

/** Where the invite's map stands, with the explicit download when possible. */
function MapState({ check }: { check: InviteCheck }) {
  const { t } = useTranslation("multi");
  const qc = useQueryClient();
  const navigate = useNavigate();
  const download = useDownloadMap();
  const progress = useDownloadProgress();
  const { map } = check.invite;
  const arena = targetLabel(map.target);
  const live = map.remoteId ? progress[`bakkesPlugins:${map.remoteId}`] : undefined;
  const percent = live?.total ? Math.round((live.downloaded / live.total) * 100) : null;

  if (check.alreadyOnArena) return <Status ok>{t("join.alreadyThere", { arena })}</Status>;
  if (check.libraryMapId) {
    return <Status ok>{t("join.inLibrary", { name: check.libraryMapName ?? map.name, arena })}</Status>;
  }
  return (
    <div className="flex flex-col gap-2">
      <Status ok={false}>{check.downloadable ? t("join.missingDownload") : t("join.missingImport")}</Status>
      {check.downloadable && map.remoteId ? (
        <Button
          size="sm"
          variant="secondary"
          loading={download.isPending}
          onClick={() =>
            map.remoteId &&
            download.mutate(
              { source: "bakkesPlugins", remoteId: map.remoteId },
              {
                onSuccess: () => {
                  notify.success(t("join.downloaded", { name: map.name }));
                  void qc.invalidateQueries({ queryKey: multiKeys.check });
                },
              },
            )
          }
        >
          <Download />
          {download.isPending && percent !== null ? t("join.downloading", { percent }) : t("join.download")}
        </Button>
      ) : (
        <Button size="sm" variant="ghost" onClick={() => void navigate({ to: "/maps" })}>
          {t("join.openMaps")}
        </Button>
      )}
    </div>
  );
}

export function JoinCard({ status, busy }: { status: MultiStatus | undefined; busy: boolean }) {
  const { t } = useTranslation("multi");
  const [code, setCode] = useState("");
  const check = useInviteCheck(code);
  const join = useJoin();
  const c = check.data;
  const blocked = !status ? t("play.checking") : !isReady(status) ? t("play.notReady") : null;

  const paste = async () => {
    try {
      setCode(await navigator.clipboard.readText());
    } catch (err) {
      notify.error(err);
    }
  };

  return (
    <Card team="blue" icon={LogIn} title={t("join.title")} description={t("join.description")}>
      <div className="flex gap-2">
        <Input
          value={code}
          onChange={(e) => setCode(e.target.value)}
          placeholder="ALXS-MULTI-…"
          spellCheck={false}
          className="font-mono text-xs"
        />
        <Button
          size="icon"
          variant="secondary"
          aria-label={t("join.paste")}
          title={t("join.paste")}
          onClick={paste}
        >
          <ClipboardPaste />
        </Button>
      </div>
      {c ? (
        <div className="flex flex-col gap-2 rounded-[8px] border border-line p-2.5 text-xs">
          <p className="flex items-center justify-between gap-2">
            <span className="font-medium text-fg">{c.invite.map.name}</span>
            <Badge tone="blue" title={t("join.arenaHint")}>
              {targetLabel(c.invite.map.target)}
            </Badge>
          </p>
          <p className="text-fg-muted">
            {t("join.from", {
              host: c.invite.hostName ?? c.invite.host,
              address: `${c.invite.host}:${c.invite.port}`,
            })}
          </p>
          <MapState check={c} />
        </div>
      ) : check.isError && code.trim() ? (
        <p className="text-xs text-danger">{toAppError(check.error).message}</p>
      ) : null}
      <div className="mt-auto flex flex-col gap-2">
        <Button
          variant="primary"
          className="w-full"
          disabled={!c?.libraryMapId || Boolean(blocked) || busy}
          loading={join.isPending}
          onClick={() => join.mutate(code.trim())}
        >
          <LogIn />
          {t("join.cta")}
        </Button>
        {blocked ? <p className="text-center text-xs text-warning">{blocked}</p> : null}
      </div>
    </Card>
  );
}
