import { Camera, Layers } from "lucide-react";
import { AnimatePresence } from "motion/react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { EmptyState, ErrorState, Skeleton } from "@/components/ui/feedback";
import { Page, PageHeader } from "@/components/ui/layout";
import { notify } from "@/components/ui/toast";
import { useCatalog } from "@/features/items/queries";
import { useGameStatus } from "@/lib/game";
import { useUi } from "@/stores/ui";
import type { Preset, PresetApplyReport } from "./api";
import { ApplyReportDialog } from "./components/ApplyReportDialog";
import { EditPresetDialog } from "./components/EditPresetDialog";
import { ImportDialog } from "./components/ImportDialog";
import { NameDialog } from "./components/NameDialog";
import { PresetCard } from "./components/PresetCard";
import { RandomDialog } from "./components/RandomDialog";
import { ShareDialog } from "./components/ShareDialog";
import { useApplyPreset, useCapturePreset, useDeletePreset, usePresets } from "./queries";

export function PresetsPage() {
  const { t } = useTranslation("presets");
  const presets = usePresets();
  const { data: catalog } = useCatalog();
  const byId = useMemo(() => new Map(catalog?.items.map((i) => [i.id, i])), [catalog]);

  const capture = useCapturePreset();
  const remove = useDeletePreset();
  const apply = useApplyPreset();
  const gameRunning = useGameStatus().data?.running.running ?? false;

  const [captureOpen, setCaptureOpen] = useState(false);
  const [editing, setEditing] = useState<Preset | null>(null);
  const [sharing, setSharing] = useState<Preset | null>(null);
  const [report, setReport] = useState<PresetApplyReport | null>(null);

  // "Edit" from the active mods panel.
  const presetToEdit = useUi((s) => s.presetToEdit);
  const editPreset = useUi((s) => s.editPreset);
  useEffect(() => {
    if (!presetToEdit || !presets.data) return;
    const preset = presets.data.find((p) => p.id === presetToEdit);
    if (preset) setEditing(preset);
    editPreset(null);
  }, [presetToEdit, presets.data, editPreset]);

  const onApply = (preset: Preset) =>
    apply.mutate(preset.id, {
      onSuccess: (r) => {
        if (r.ok) notify.success(t("toast.applied", { name: preset.name }));
        else setReport(r);
      },
    });

  return (
    <Page>
      <PageHeader
        icon={Layers}
        eyebrow={t("eyebrow")}
        title={t("title")}
        subtitle={t("subtitle")}
        actions={
          <>
            <ImportDialog />
            <RandomDialog />
            <Button variant="primary" onClick={() => setCaptureOpen(true)}>
              <Camera />
              {t("capture.open")}
            </Button>
          </>
        }
      />

      {presets.isLoading ? (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-4">
          {[0, 1, 2].map((i) => (
            <Skeleton key={i} className="h-64" />
          ))}
        </div>
      ) : presets.error ? (
        <ErrorState error={presets.error} onRetry={() => void presets.refetch()} />
      ) : !presets.data?.length ? (
        <EmptyState
          icon={Layers}
          title={t("empty.title")}
          description={t("empty.description")}
          action={
            <Button variant="primary" onClick={() => setCaptureOpen(true)}>
              <Camera />
              {t("capture.open")}
            </Button>
          }
        />
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-4">
          <AnimatePresence>
            {presets.data.map((preset, i) => (
              <PresetCard
                key={preset.id}
                preset={preset}
                index={i}
                byId={byId}
                applying={apply.isPending && apply.variables === preset.id}
                gameRunning={gameRunning}
                onApply={() => onApply(preset)}
                onShare={() => setSharing(preset)}
                onEdit={() => setEditing(preset)}
                onDelete={() => remove.mutate(preset.id)}
              />
            ))}
          </AnimatePresence>
        </div>
      )}

      <NameDialog
        open={captureOpen}
        onOpenChange={setCaptureOpen}
        title={t("capture.title")}
        description={t("capture.description")}
        confirmLabel={t("capture.confirm")}
        pending={capture.isPending}
        onConfirm={(name) => capture.mutate(name, { onSuccess: () => setCaptureOpen(false) })}
      />
      <EditPresetDialog preset={editing} byId={byId} onClose={() => setEditing(null)} />
      <ShareDialog preset={sharing} onClose={() => setSharing(null)} />
      <ApplyReportDialog report={report} onClose={() => setReport(null)} />
    </Page>
  );
}
