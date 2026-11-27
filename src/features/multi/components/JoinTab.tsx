import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { ClipboardPaste, Download, LogIn } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { GlassPanel } from "@/components/ui/glass";
import { Input } from "@/components/ui/input";
import { notify } from "@/components/ui/toast";
import { targetLabel } from "@/features/maps/api";
import { useDownloadMap, useDownloadProgress } from "@/features/maps/queries";
import { toAppError } from "@/lib/errors";
import type { InviteCheck, MultiStatus } from "../api";
import { multiKeys, useInviteCheck, useJoin } from "../queries";
import { isReady } from "./SetupPanel";
import { Status } from "./Status";

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
    <div className="flex flex-col items-start gap-2">
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

export function JoinTab({
  status,
  busy,
  onShowInfos,
}: {
  status: MultiStatus | undefined;
  busy: boolean;
  onShowInfos: () => void;
}) {
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
    <GlassPanel
      className="mx-auto flex w-full max-w-2xl flex-col gap-4 p-4"
      style={{ ["--color-accent" as string]: "var(--color-team-blue)" }}
    >
      <div>
        <h3 className="flex items-center gap-2 text-sm font-semibold">
          <LogIn className="size-4" style={{ color: "var(--color-team-blue)" }} />
          {t("join.title")}
        </h3>
        <p className="mt-1 text-[13px] text-fg-muted">{t("join.description")}</p>
      </div>
      <div className="flex gap-2">
        <Input
          value={code}
          onChange={(e) => setCode(e.target.value)}
          placeholder="ALXS-MULTI-…"
          spellCheck={false}
          className="font-mono text-xs"
        />
        <Button variant="secondary" onClick={paste}>
          <ClipboardPaste />
          {t("join.paste")}
        </Button>
      </div>
      {c ? (
        <div className="flex flex-col gap-2 rounded-[8px] border border-line p-3 text-xs">
          <p className="flex items-center justify-between gap-2">
            <span className="text-[13px] font-medium text-fg">{c.invite.map.name}</span>
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
      {blocked ? (
        <button
          type="button"
          onClick={onShowInfos}
          className="text-center text-xs text-warning hover:underline"
        >
          {blocked}
        </button>
      ) : null}
    </GlassPanel>
  );
}
