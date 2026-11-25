import { ClipboardPaste, LogIn, Radio } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { GlassCard } from "@/components/ui/glass";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { notify } from "@/components/ui/toast";
import { targetLabel } from "@/features/maps/api";
import { useMaps } from "@/features/maps/queries";
import { toAppError } from "@/lib/errors";
import type { MultiStatus } from "../api";
import { useDecodeInvite, useHost, useJoin } from "../queries";
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
  children: React.ReactNode;
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

export function HostCard({ status, busy }: { status: MultiStatus | undefined; busy: boolean }) {
  const { t } = useTranslation("multi");
  const maps = useMaps();
  const host = useHost();
  const [mapId, setMapId] = useState<string | undefined>(undefined);
  const list = maps.data ?? [];
  const selected = list.find((m) => m.id === mapId) ?? list[0];
  const blocked = !status ? t("play.checking") : !isReady(status) ? t("play.notReady") : null;

  return (
    <Card team="orange" icon={Radio} title={t("host.title")} description={t("host.description")}>
      {list.length ? (
        <Select
          value={selected?.id}
          onValueChange={setMapId}
          options={list.map((m) => ({ value: m.id, label: m.name, hint: m.favorite ? "★" : undefined }))}
          placeholder={t("host.pickMap")}
        />
      ) : (
        <p className="text-xs text-fg-muted">{t("host.noMaps")}</p>
      )}
      {selected ? (
        <p className="text-xs text-fg-subtle">
          {t("host.target", { arena: targetLabel(selected.preferredTarget ?? "Labs_Underpass_P.upk") })}
        </p>
      ) : null}
      <div className="mt-auto flex flex-col gap-2">
        <Button
          variant="primary"
          className="w-full"
          disabled={!selected || Boolean(blocked) || busy}
          loading={host.isPending}
          onClick={() => selected && host.mutate(selected.id)}
        >
          <Radio />
          {t("host.cta")}
        </Button>
        {blocked ? <p className="text-center text-xs text-warning">{blocked}</p> : null}
      </div>
    </Card>
  );
}

export function JoinCard({ status, busy }: { status: MultiStatus | undefined; busy: boolean }) {
  const { t } = useTranslation("multi");
  const [code, setCode] = useState("");
  const decoded = useDecodeInvite(code);
  const join = useJoin();
  const invite = decoded.data;
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
      {invite ? (
        <div className="flex flex-col gap-1 rounded-[8px] border border-line p-2.5 text-xs">
          <p className="flex items-center justify-between gap-2">
            <span className="font-medium text-fg">{invite.map.name}</span>
            <Badge tone="blue">{targetLabel(invite.map.target)}</Badge>
          </p>
          <p className="text-fg-muted">
            {t("join.from", {
              host: invite.hostName ?? invite.host,
              address: `${invite.host}:${invite.port}`,
            })}
          </p>
          <p className="text-fg-subtle">
            {invite.map.source === "bakkesPlugins" ? t("join.autoDownload") : t("join.needsFile")}
          </p>
        </div>
      ) : decoded.isError && code.trim() ? (
        <p className="text-xs text-danger">{toAppError(decoded.error).message}</p>
      ) : null}
      <div className="mt-auto flex flex-col gap-2">
        <Button
          variant="primary"
          className="w-full"
          disabled={!invite || Boolean(blocked) || busy}
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
