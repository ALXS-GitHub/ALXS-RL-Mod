import { open } from "@tauri-apps/plugin-dialog";
import { CheckCircle2, FileKey2, ShieldAlert } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { notify } from "@/components/ui/toast";
import { useImportKeys, useKeysStatus } from "@/features/settings/api";
import { useConfig, useUpdateConfig } from "@/lib/game";

/** Picks a keys.txt and imports it; shared by the welcome dialog and Settings. */
export function useKeysPicker() {
  const { t } = useTranslation();
  const importKeys = useImportKeys();
  const pick = async () => {
    const path = await open({
      multiple: false,
      directory: false,
      title: t("welcome.keys.pick"),
      filters: [{ name: "keys.txt", extensions: ["txt"] }],
    });
    if (typeof path !== "string") return;
    importKeys.mutate(path, {
      onSuccess: (s) => notify.success(t("welcome.keys.imported", { count: s.count })),
      onError: notify.error,
    });
  };
  return { pick, busy: importKeys.isPending };
}

/**
 * First launch: what the app does to the game (and the risks), then the
 * package keys it needs but cannot ship.
 */
export function WelcomeDialog() {
  const { t } = useTranslation();
  const config = useConfig();
  const update = useUpdateConfig();
  const keys = useKeysStatus();
  const { pick, busy } = useKeysPicker();
  const [step, setStep] = useState<"warning" | "keys">("warning");
  const [accepted, setAccepted] = useState(false);

  const open = config.data ? !config.data.onboardingDone : false;
  const finish = () => update.mutate({ onboardingDone: true });

  return (
    <Dialog open={open}>
      <DialogContent
        size="md"
        title={step === "warning" ? t("welcome.title") : t("welcome.keys.title")}
        description={step === "warning" ? t("welcome.subtitle") : t("welcome.keys.subtitle")}
        onEscapeKeyDown={(e) => e.preventDefault()}
        onPointerDownOutside={(e) => e.preventDefault()}
        footer={
          step === "warning" ? (
            <Button variant="primary" disabled={!accepted} onClick={() => setStep("keys")}>
              {t("welcome.continue")}
            </Button>
          ) : (
            <>
              <Button variant="ghost" onClick={finish} loading={update.isPending}>
                {keys.data?.present ? t("welcome.done") : t("welcome.later")}
              </Button>
              {keys.data?.present ? null : (
                <Button variant="primary" loading={busy} onClick={() => void pick()}>
                  <FileKey2 />
                  {t("welcome.keys.import")}
                </Button>
              )}
            </>
          )
        }
      >
        {step === "warning" ? (
          <div className="flex flex-col gap-3 text-[13px] text-fg-muted">
            <ul className="flex list-disc flex-col gap-1.5 pl-5">
              <li>{t("welcome.points.files")}</li>
              <li>{t("welcome.points.local")}</li>
              <li>{t("welcome.points.restore")}</li>
              <li>{t("welcome.points.affiliation")}</li>
            </ul>
            <div className="flex items-start gap-2.5 rounded-[8px] border border-warning/25 bg-warning/[0.06] p-2.5 text-xs text-fg">
              <ShieldAlert className="mt-0.5 size-3.5 shrink-0 text-warning" />
              <p>{t("welcome.risk")}</p>
            </div>
            <label className="flex cursor-pointer items-center gap-2.5 text-fg">
              <input
                type="checkbox"
                className="size-4 accent-[var(--color-accent)]"
                checked={accepted}
                onChange={(e) => setAccepted(e.target.checked)}
              />
              {t("welcome.accept")}
            </label>
          </div>
        ) : (
          <div className="flex flex-col gap-3 text-[13px] text-fg-muted">
            <p>{t("welcome.keys.why")}</p>
            <p>{t("welcome.keys.where")}</p>
            <p>{t("welcome.keys.updates")}</p>
            <div className="flex items-center gap-2">
              {keys.data?.present ? (
                <Badge tone="success" dot>
                  <CheckCircle2 className="size-3" />
                  {t("welcome.keys.present", { count: keys.data.count })}
                </Badge>
              ) : (
                <Badge tone="warning" dot>
                  {t("welcome.keys.missing")}
                </Badge>
              )}
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
