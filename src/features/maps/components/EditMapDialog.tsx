import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { LABS_TARGETS, type MapEntry, targetLabel } from "../api";
import { useUpdateMap } from "../queries";

const DEFAULT_OPTION = "__default";

interface EditMapDialogProps {
  map: MapEntry | null;
  onClose: () => void;
}

export function EditMapDialog({ map, onClose }: EditMapDialogProps) {
  const { t } = useTranslation("maps");
  const { t: tc } = useTranslation();
  const update = useUpdateMap();
  const [name, setName] = useState("");
  const [tags, setTags] = useState("");
  const [target, setTarget] = useState<string>(DEFAULT_OPTION);

  useEffect(() => {
    if (!map) return;
    setName(map.name);
    setTags(map.tags.join(", "));
    setTarget(map.preferredTarget ?? DEFAULT_OPTION);
  }, [map]);

  const save = () => {
    if (!map) return;
    update.mutate(
      {
        id: map.id,
        patch: {
          name: name.trim(),
          tags: tags
            .split(",")
            .map((s) => s.trim())
            .filter(Boolean),
          preferredTarget: target === DEFAULT_OPTION ? null : target,
        },
      },
      { onSuccess: onClose },
    );
  };

  const options = [
    { value: DEFAULT_OPTION, label: t("edit.defaultTarget", { target: targetLabel(LABS_TARGETS[0]) }) },
    ...LABS_TARGETS.map((v) => ({ value: v, label: targetLabel(v) })),
  ];

  return (
    <Dialog open={map !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        title={t("edit.title")}
        description={map?.name}
        footer={
          <>
            <Button variant="ghost" onClick={onClose}>
              {tc("actions.cancel")}
            </Button>
            <Button variant="primary" onClick={save} loading={update.isPending} disabled={!name.trim()}>
              {tc("actions.save")}
            </Button>
          </>
        }
      >
        <div className="flex flex-col gap-4">
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-fg-muted">{t("edit.name")}</span>
            <Input value={name} onChange={(e) => setName(e.target.value)} maxLength={120} autoFocus />
          </label>
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-fg-muted">{t("edit.tags")}</span>
            <Input
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              placeholder={t("edit.tagsPlaceholder")}
            />
          </label>
          <div className="flex flex-col gap-1.5 text-sm">
            <span className="text-fg-muted">{t("edit.target")}</span>
            <Select value={target} onValueChange={setTarget} options={options} />
            <span className="text-xs text-fg-subtle">{t("edit.targetHint")}</span>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
