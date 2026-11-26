import { Check, CircleAlert, Copy, Loader2, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { GlassPanel } from "@/components/ui/glass";
import { notify } from "@/components/ui/toast";
import { targetLabel } from "@/features/maps/api";
import { cn } from "@/lib/cn";
import type { MultiSession, Phase } from "../api";
import { useCancelSession } from "../queries";

/** Automatic steps, in order. */
const STEPS: readonly Phase[] = [
  "installingMap",
  "launchingGame",
  "startingBakkes",
  "waitingBakkes",
  "commandSent",
];

function stepIndex(phase: Phase): number {
  // Waiting for the user in Epic is still the "start the game" step.
  if (phase === "launchInEpic") return STEPS.indexOf("launchingGame");
  if (phase === "hostInRocketPlugin") return STEPS.indexOf("commandSent");
  return STEPS.indexOf(phase);
}

function CopyBox({ label, value }: { label: string; value: string }) {
  const { t } = useTranslation("multi");
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(value);
      notify.success(t("session.copied"));
    } catch (err) {
      notify.error(err);
    }
  };
  return (
    <div className="flex flex-col gap-1.5">
      <p className="text-xs text-fg-subtle">{label}</p>
      <div className="flex items-center gap-2">
        <code className="min-w-0 flex-1 truncate rounded-[7px] border border-line bg-black/25 px-2.5 py-1.5 font-mono text-xs">
          {value}
        </code>
        <Button size="sm" variant="secondary" onClick={copy}>
          <Copy />
          {t("session.copy")}
        </Button>
      </div>
    </div>
  );
}

export function SessionPanel({ session }: { session: MultiSession }) {
  const { t } = useTranslation("multi");
  const cancel = useCancelSession();
  const current = stepIndex(session.phase);
  const stopped = session.phase === "manual" || session.phase === "failed";
  const byHand = session.phase === "hostInRocketPlugin";
  const done = session.phase === "commandSent" || byHand;

  return (
    <GlassPanel className="flex flex-col gap-4 p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <Badge tone={session.role === "host" ? "orange" : "blue"}>
            {t(`session.role.${session.role}`)}
          </Badge>
          <h2 className="text-sm font-semibold">{session.mapName}</h2>
          <span className="text-xs text-fg-subtle">{targetLabel(session.target)}</span>
        </div>
        <Button size="sm" variant="ghost" loading={cancel.isPending} onClick={() => cancel.mutate()}>
          <X />
          {done || stopped ? t("session.close") : t("session.cancel")}
        </Button>
      </div>

      <ol className="flex flex-wrap gap-x-5 gap-y-2">
        {STEPS.map((step, i) => {
          const isDone = done || i < current;
          const isCurrent = !done && !stopped && i === current;
          const label =
            step === "launchingGame" && session.phase === "launchInEpic"
              ? t("session.phase.launchInEpic")
              : step === "commandSent" && byHand
                ? t("session.phase.hostInRocketPlugin")
                : t(`session.phase.${step}`);
          return (
            <li
              key={step}
              className={cn(
                "flex items-center gap-1.5 text-xs",
                isDone ? "text-fg" : isCurrent ? "text-fg" : "text-fg-subtle",
              )}
            >
              {isDone ? (
                <Check className="size-3.5 text-success" />
              ) : isCurrent ? (
                <Loader2 className="size-3.5 animate-spin" />
              ) : (
                <span className="size-1.5 rounded-full bg-fg-subtle" />
              )}
              {label}
            </li>
          );
        })}
      </ol>

      {session.message ? (
        <div
          className={cn(
            "flex items-start gap-2.5 rounded-[8px] border p-2.5 text-xs text-fg-muted",
            session.phase === "failed"
              ? "border-danger/25 bg-danger/[0.06]"
              : "border-warning/25 bg-warning/[0.06]",
          )}
        >
          <CircleAlert
            className={cn(
              "mt-0.5 size-3.5 shrink-0",
              session.phase === "failed" ? "text-danger" : "text-warning",
            )}
          />
          <div>
            <p className="font-medium text-fg">
              {session.phase === "failed" ? t("session.stopped.failed") : t("session.stopped.manual")}
            </p>
            <p className="mt-0.5">{session.message}</p>
          </div>
        </div>
      ) : null}

      {session.phase === "launchInEpic" ? (
        <div className="flex items-start gap-2.5 rounded-[8px] border border-info/25 bg-info/[0.06] p-2.5 text-xs text-fg-muted">
          <CircleAlert className="mt-0.5 size-3.5 shrink-0 text-info" />
          <div>
            <p className="font-medium text-fg">{t("session.epic.title")}</p>
            <p className="mt-0.5">{t("session.epic.hint")}</p>
          </div>
        </div>
      ) : null}

      {byHand ? (
        <div className="flex items-start gap-2.5 rounded-[8px] border border-info/25 bg-info/[0.06] p-2.5 text-xs text-fg-muted">
          <CircleAlert className="mt-0.5 size-3.5 shrink-0 text-info" />
          <div>
            <p className="font-medium text-fg">{t("session.byHand.title")}</p>
            <p className="mt-0.5">{t("session.byHand.hint", { arena: targetLabel(session.target) })}</p>
          </div>
        </div>
      ) : null}

      {session.mapMismatch ? (
        <div className="flex items-start gap-2.5 rounded-[8px] border border-warning/25 bg-warning/[0.06] p-2.5 text-xs text-fg-muted">
          <CircleAlert className="mt-0.5 size-3.5 shrink-0 text-warning" />
          <p>{t("session.mismatch")}</p>
        </div>
      ) : null}

      {session.role === "host" && session.inviteCode ? (
        <CopyBox label={t("session.invite")} value={session.inviteCode} />
      ) : null}
      {session.address ? (
        <p className="text-xs text-fg-muted">
          {session.role === "host"
            ? t("session.hostAddress", { address: session.address })
            : t("session.joinAddress", { address: session.address })}
        </p>
      ) : null}
      {session.command && (stopped || done) ? (
        <CopyBox
          label={session.phase === "manual" ? t("session.manual") : t("session.again")}
          value={session.command}
        />
      ) : null}
      {done && !byHand ? (
        <p className="text-xs text-fg-muted">
          {session.role === "host" ? t("session.hostDone") : t("session.joinDone")}
        </p>
      ) : null}
    </GlassPanel>
  );
}
