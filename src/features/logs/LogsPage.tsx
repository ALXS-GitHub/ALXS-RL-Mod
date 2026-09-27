import { Link } from "@tanstack/react-router";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { ArrowLeft, FolderOpen, ScrollText, Trash2 } from "lucide-react";
import { useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { EmptyState, ErrorState, Skeleton } from "@/components/ui/feedback";
import { SearchInput } from "@/components/ui/input";
import { Page, PageHeader } from "@/components/ui/layout";
import { Segmented } from "@/components/ui/segmented";
import { Select } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { notify } from "@/components/ui/toast";
import { cn } from "@/lib/cn";
import { formatBytes, formatRelative } from "@/lib/format";
import { type LogLevel, type LogLine, type LogSource, useCleanLogs, useLogs } from "./api";

type LevelFilter = "all" | "warn" | "error";

const LEVEL_STYLE: Record<LogLevel, string> = {
  error: "text-danger",
  warn: "text-warning",
  info: "text-fg-subtle",
  debug: "text-fg-subtle/70",
};

function keep(line: LogLine, filter: LevelFilter): boolean {
  if (filter === "error") return line.level === "error";
  if (filter === "warn") return line.level === "error" || line.level === "warn";
  return true;
}

/** Short time: `08:52:43` for app logs, `0024.47` (seconds since launch) for the game. */
function shortTime(time: string | null): string {
  if (!time) return "";
  const m = /T(\d{2}:\d{2}:\d{2})/.exec(time);
  return m?.[1] ?? time;
}

function Row({ line }: { line: LogLine }) {
  return (
    <li
      className={cn(
        "grid grid-cols-[64px_44px_minmax(0,180px)_minmax(0,1fr)] gap-3 px-3 py-[3px] hover:bg-white/[0.03]",
        line.level === "error" && "bg-danger/[0.06]",
        line.level === "warn" && "bg-warning/[0.04]",
      )}
    >
      <span className="text-fg-subtle tabular-nums">{shortTime(line.time)}</span>
      <span className={LEVEL_STYLE[line.level]}>{line.time ? line.level : ""}</span>
      <span className="truncate text-fg-subtle" title={line.target ?? undefined}>
        {line.target?.replace(/^alxs_rl_mod_lib::/, "") ?? ""}
      </span>
      <span className={cn("break-words whitespace-pre-wrap", line.level === "error" && "text-danger")}>
        {line.message}
      </span>
    </li>
  );
}

export function LogsPage() {
  const { t } = useTranslation("logs");
  const [source, setSource] = useState<LogSource>("app");
  const [file, setFile] = useState<string | null>(null);
  const [level, setLevel] = useState<LevelFilter>("all");
  const [search, setSearch] = useState("");
  const [follow, setFollow] = useState(true);
  const [confirmClean, setConfirmClean] = useState(false);
  const deferredSearch = useDeferredValue(search);
  const logs = useLogs(source, file, follow);
  const clean = useCleanLogs();
  const listRef = useRef<HTMLDivElement>(null);

  const visible = useMemo(() => {
    const q = deferredSearch.trim().toLowerCase();
    return (logs.data?.lines ?? []).filter(
      (l) =>
        keep(l, level) && (!q || l.message.toLowerCase().includes(q) || l.target?.toLowerCase().includes(q)),
    );
  }, [logs.data, level, deferredSearch]);

  const counts = useMemo(() => {
    const lines = logs.data?.lines ?? [];
    return {
      error: lines.filter((l) => l.level === "error").length,
      warn: lines.filter((l) => l.level === "warn").length,
    };
  }, [logs.data]);

  // Newest lines are at the bottom: stick to it while following.
  const lineCount = visible.length;
  useEffect(() => {
    const el = listRef.current;
    if (follow && el && lineCount > 0) el.scrollTop = el.scrollHeight;
  }, [follow, lineCount]);

  const changeSource = (next: LogSource) => {
    setSource(next);
    setFile(null);
  };

  const fileOptions = (logs.data?.files ?? []).map((f) => ({
    value: f.name,
    label: `${f.name} · ${formatBytes(f.size)}${f.modified ? ` · ${formatRelative(f.modified)}` : ""}`,
  }));

  return (
    <Page className="max-w-none">
      <PageHeader
        icon={ScrollText}
        eyebrow={
          <Link to="/settings" className="flex items-center gap-1 hover:text-fg">
            <ArrowLeft className="size-3.5" />
            {t("back")}
          </Link>
        }
        title={t("title")}
        subtitle={t("subtitle")}
        actions={
          <>
            <Button
              variant="ghost"
              disabled={!logs.data?.folder}
              onClick={() => logs.data?.folder && void revealItemInDir(logs.data.folder)}
            >
              <FolderOpen />
              {t("common:actions.openFolder")}
            </Button>
            {source === "app" ? (
              <Button variant="secondary" onClick={() => setConfirmClean(true)}>
                <Trash2 />
                {t("clean.action")}
              </Button>
            ) : null}
          </>
        }
      />

      <div className="flex flex-wrap items-center gap-2">
        <Segmented
          value={source}
          onValueChange={changeSource}
          options={[
            { value: "app", label: t("source.app") },
            { value: "game", label: t("source.game") },
          ]}
        />
        <Select
          value={logs.data?.file ?? ""}
          onValueChange={setFile}
          options={fileOptions}
          placeholder={t("noFile")}
          className="w-80"
          disabled={fileOptions.length === 0}
        />
        <Segmented
          value={level}
          onValueChange={setLevel}
          size="sm"
          options={[
            { value: "all", label: t("level.all") },
            { value: "warn", label: t("level.warn", { count: counts.warn + counts.error }) },
            { value: "error", label: t("level.error", { count: counts.error }) },
          ]}
        />
        <SearchInput
          value={search}
          onValueChange={setSearch}
          placeholder={t("search")}
          className="min-w-56 flex-1"
        />
        <label className="flex items-center gap-2 text-xs text-fg-muted">
          <Switch checked={follow} onCheckedChange={setFollow} />
          {t("follow")}
        </label>
      </div>

      {logs.error ? (
        <ErrorState error={logs.error} onRetry={() => void logs.refetch()} />
      ) : logs.isPending ? (
        <Skeleton className="h-[60vh]" />
      ) : !logs.data?.file ? (
        <EmptyState icon={ScrollText} title={t("empty.title")} description={t("empty.description")} />
      ) : (
        <div className="glass flex min-h-0 flex-col rounded-lg">
          <div className="flex items-center justify-between border-line border-b px-3 py-2 text-[11px] text-fg-subtle">
            <span>
              {t("shown", { count: visible.length, total: logs.data.lines.length })}
              {logs.data.truncated ? ` · ${t("truncated")}` : ""}
            </span>
            <span className="truncate pl-3">{logs.data.file}</span>
          </div>
          <div ref={listRef} className="h-[calc(100vh-19rem)] min-h-[360px] overflow-y-auto py-1">
            {visible.length === 0 ? (
              <p className="px-3 py-8 text-center text-xs text-fg-subtle">{t("noMatch")}</p>
            ) : (
              <ul className="font-mono text-[12px] leading-[18px]">
                {visible.map((line, i) => (
                  // biome-ignore lint/suspicious/noArrayIndexKey: log lines have no id; the list is append-only
                  <Row key={i} line={line} />
                ))}
              </ul>
            )}
          </div>
        </div>
      )}

      <Dialog open={confirmClean} onOpenChange={setConfirmClean}>
        <DialogContent
          size="sm"
          title={t("clean.title")}
          description={t("clean.body")}
          footer={
            <>
              <Button variant="ghost" onClick={() => setConfirmClean(false)}>
                {t("common:actions.cancel")}
              </Button>
              <Button
                variant="danger"
                loading={clean.isPending}
                onClick={() =>
                  clean.mutate(undefined, {
                    onSuccess: (count) => {
                      setConfirmClean(false);
                      setFile(null);
                      notify.success(t("clean.done", { count }));
                    },
                    onError: notify.error,
                  })
                }
              >
                <Trash2 />
                {t("clean.action")}
              </Button>
            </>
          }
        >
          <p className="text-xs text-fg-subtle">{t("clean.retention")}</p>
        </DialogContent>
      </Dialog>
    </Page>
  );
}
