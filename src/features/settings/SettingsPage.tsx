import { Link } from "@tanstack/react-router";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  Cpu,
  FileArchive,
  FileKey2,
  FolderOpen,
  FolderSearch,
  KeyRound,
  Languages,
  RefreshCcw,
  RefreshCw,
  ScrollText,
  Settings,
  ShieldCheck,
  Sparkles,
  Wand2,
} from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useKeysPicker } from "@/components/shell/WelcomeDialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ErrorState, Skeleton } from "@/components/ui/feedback";
import { GlassCard } from "@/components/ui/glass";
import { Field, Page, PageHeader } from "@/components/ui/layout";
import { Segmented } from "@/components/ui/segmented";
import { Switch } from "@/components/ui/switch";
import { notify } from "@/components/ui/toast";
import {
  type AppConfigPatch,
  useAppInfo,
  useConfig,
  useGameStatus,
  useSetInstallDir,
  useUpdateConfig,
} from "@/lib/game";
import { useUi } from "@/stores/ui";
import { useExportDiagnostics, useKeysStatus } from "./api";
import { RestoreStockField } from "./components/RestoreStock";

function Card({
  icon: Icon,
  title,
  children,
}: {
  icon: typeof Settings;
  title: string;
  children: ReactNode;
}) {
  return (
    <GlassCard interactive={false} className="px-4 pt-3.5 pb-1">
      <div className="mb-1 flex items-center gap-2">
        <Icon className="size-4 text-fg-subtle" />
        <h2 className="text-sm font-semibold">{title}</h2>
      </div>
      <div className="divide-y divide-line">{children}</div>
    </GlassCard>
  );
}

export function SettingsPage() {
  const { t } = useTranslation("settings");
  const config = useConfig();
  const game = useGameStatus();
  const info = useAppInfo();
  const keys = useKeysStatus();
  const keysPicker = useKeysPicker();
  const requestUpdateCheck = useUi((st) => st.requestUpdateCheck);
  const update = useUpdateConfig();
  const setDir = useSetInstallDir();
  const diagnostics = useExportDiagnostics();

  const patch = (p: AppConfigPatch) => update.mutate(p, { onError: notify.error });

  const pickFolder = async () => {
    const picked = await open({ directory: true, multiple: false, title: t("game.pickTitle") });
    if (typeof picked !== "string") return;
    setDir.mutate(picked, {
      onSuccess: () => notify.success(t("game.pickSuccess")),
      onError: notify.error,
    });
  };

  if (config.isError)
    return (
      <Page>
        <ErrorState error={config.error} onRetry={() => void config.refetch()} />
      </Page>
    );
  const cfg = config.data;
  const install = game.data?.install;

  return (
    <Page>
      <PageHeader icon={Settings} eyebrow={t("eyebrow")} title={t("title")} subtitle={t("subtitle")} />

      <div className="grid gap-5 @4xl:grid-cols-2">
        <Card icon={FolderSearch} title={t("game.title")}>
          <div className="py-3.5">
            {game.isLoading ? (
              <Skeleton className="h-12" />
            ) : install ? (
              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <Badge tone="success" dot>
                      {t(`game.source.${install.source}`)}
                    </Badge>
                  </div>
                  <p
                    className="mt-2 truncate font-mono text-xs text-fg-muted"
                    data-selectable
                    title={install.root}
                  >
                    {install.root}
                  </p>
                </div>
                <Button size="sm" variant="ghost" onClick={() => void revealItemInDir(install.root)}>
                  <FolderOpen />
                  {t("common:actions.openFolder")}
                </Button>
              </div>
            ) : (
              <Badge tone="warning" dot>
                {t("common:game.notFound")}
              </Badge>
            )}
          </div>
          <Field label={t("game.folder")} description={t("game.folderHint")}>
            <div className="flex gap-2">
              {cfg?.rlInstallOverride ? (
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setDir.mutate(null, { onError: notify.error })}
                >
                  <RefreshCcw />
                  {t("game.autoDetect")}
                </Button>
              ) : null}
              <Button size="sm" onClick={() => void pickFolder()} loading={setDir.isPending}>
                {t("common:actions.browse")}
              </Button>
            </div>
          </Field>
        </Card>

        <Card icon={KeyRound} title={t("keys.title")}>
          <Field
            label={t("keys.status")}
            description={
              keys.data?.present ? t("keys.presentHint", { count: keys.data.count }) : t("keys.missingHint")
            }
          >
            {keys.isLoading ? (
              <Skeleton className="h-6 w-20" />
            ) : keys.data?.present ? (
              <Badge tone="success" dot>
                {t("keys.present")}
              </Badge>
            ) : (
              <Badge tone="warning" dot>
                {t("keys.missing")}
              </Badge>
            )}
          </Field>
          <Field label={t("keys.dropIn")} description={t("keys.dropInHint")}>
            <Button
              size="sm"
              variant="secondary"
              loading={keysPicker.busy}
              onClick={() => void keysPicker.pick()}
            >
              <FileKey2 />
              {t("keys.import")}
            </Button>
            <Button
              size="sm"
              variant="ghost"
              disabled={!info.data}
              onClick={() => info.data && void revealItemInDir(info.data.dataDir)}
            >
              <FolderOpen />
              {t("common:actions.openFolder")}
            </Button>
          </Field>
          <RestoreStockField />
        </Card>

        <Card icon={Sparkles} title={t("appearance.title")}>
          <Field label={t("appearance.language")} description={t("appearance.languageHint")}>
            <Segmented
              aria-label={t("appearance.language")}
              size="sm"
              value={cfg?.locale ?? "en"}
              onValueChange={(locale) => patch({ locale })}
              options={[
                { value: "en", label: "English", icon: <Languages /> },
                { value: "fr", label: "Français" },
              ]}
            />
          </Field>
          <Field label={t("appearance.fx")} description={t("appearance.fxHint")}>
            <Segmented
              aria-label={t("appearance.fx")}
              size="sm"
              value={cfg?.fxQuality ?? "high"}
              onValueChange={(fxQuality) => patch({ fxQuality })}
              options={[
                { value: "off", label: t("appearance.fxOff") },
                { value: "low", label: t("appearance.fxLow") },
                { value: "high", label: t("appearance.fxHigh") },
              ]}
            />
          </Field>
          <Field label={t("appearance.pauseInGame")} description={t("appearance.pauseInGameHint")}>
            <Switch
              checked={cfg?.fxPauseWhenGameFocused ?? true}
              onCheckedChange={(v) => patch({ fxPauseWhenGameFocused: v })}
            />
          </Field>
        </Card>

        <Card icon={Wand2} title={t("automation.title")}>
          <Field label={t("automation.reapply")} description={t("automation.reapplyHint")}>
            <Switch
              checked={cfg?.autoReapplyAfterUpdate ?? true}
              onCheckedChange={(v) => patch({ autoReapplyAfterUpdate: v })}
            />
          </Field>
          <Field label={t("automation.restoreMaps")} description={t("automation.restoreMapsHint")}>
            <Switch
              checked={cfg?.restoreMapsOnGameExit ?? true}
              onCheckedChange={(v) => patch({ restoreMapsOnGameExit: v })}
            />
          </Field>
        </Card>

        <Card icon={ShieldCheck} title={t("privacy.title")}>
          <ul className="space-y-2 py-3.5 text-sm text-fg-muted">
            {(["local", "noInjection", "noInterception", "reversible"] as const).map((k) => (
              <li key={k} className="flex gap-2">
                <ShieldCheck className="mt-0.5 size-4 shrink-0 text-success" />
                <span>{t(`privacy.${k}`)}</span>
              </li>
            ))}
          </ul>
          <Field label={t("privacy.diagnostics")} description={t("privacy.diagnosticsHint")}>
            <Button
              size="sm"
              loading={diagnostics.isPending}
              onClick={() =>
                diagnostics.mutate(undefined, {
                  onSuccess: (path) => {
                    notify.success(t("privacy.diagnosticsDone"));
                    void revealItemInDir(path);
                  },
                  onError: notify.error,
                })
              }
            >
              <FileArchive />
              {t("common:actions.export")}
            </Button>
          </Field>
        </Card>

        <Card icon={Cpu} title={t("about.title")}>
          <Field label={t("about.version")}>
            <span className="font-mono text-sm tabular-nums text-fg-muted">
              {info.data ? `v${info.data.version}${info.data.debug ? " · dev" : ""}` : "…"}
            </span>
            <Button size="sm" variant="secondary" onClick={requestUpdateCheck}>
              <RefreshCw />
              {t("about.checkUpdates")}
            </Button>
          </Field>
          <Field label={t("about.data")} description={info.data?.dataDir}>
            <Button
              size="sm"
              variant="ghost"
              disabled={!info.data}
              onClick={() => info.data && void revealItemInDir(info.data.dataDir)}
            >
              <FolderOpen />
              {t("common:actions.openFolder")}
            </Button>
          </Field>
          <Field label={t("about.logs")} description={t("about.logsHint")}>
            <div className="flex gap-1.5">
              <Button size="sm" variant="secondary" asChild>
                <Link to="/logs">
                  <ScrollText />
                  {t("about.showLogs")}
                </Link>
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={!info.data}
                onClick={() => info.data && void revealItemInDir(info.data.logsDir)}
              >
                <FolderOpen />
                {t("common:actions.openFolder")}
              </Button>
            </div>
          </Field>
        </Card>
      </div>
    </Page>
  );
}
