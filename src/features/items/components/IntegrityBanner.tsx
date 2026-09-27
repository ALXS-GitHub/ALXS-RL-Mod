import { RefreshCw, ShieldAlert } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useIntegrityCheck, useIntegrityReapply } from "../queries";

/** Shown when a game update / "verify files" undid some of our changes. */
export function IntegrityBanner() {
  const { t } = useTranslation("items");
  const { data } = useIntegrityCheck();
  const reapply = useIntegrityReapply();
  const visible = Boolean(data && (data.pending.length > 0 || data.gameUpdated));

  return (
    <AnimatePresence>
      {visible && data ? (
        <motion.div
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: "auto" }}
          exit={{ opacity: 0, height: 0 }}
          className="overflow-hidden"
        >
          <div className="flex items-center gap-3 rounded-lg border border-warning/25 bg-warning/[0.06] px-3.5 py-2.5">
            <ShieldAlert className="size-4 shrink-0 text-warning" />
            <div className="min-w-0 flex-1">
              <p className="text-[13px] font-medium">
                {data.gameUpdated ? t("integrity.updatedTitle") : t("integrity.revertedTitle")}
              </p>
              <p className="text-xs text-fg-muted">
                {t(data.gameUpdated ? "integrity.updatedDescription" : "integrity.description", {
                  count: data.filesReverted,
                  features: data.pending
                    .map((f) => t(`integrity.feature.${f}`, { defaultValue: f }))
                    .join(", "),
                })}
              </p>
            </div>
            <Button variant="primary" size="sm" loading={reapply.isPending} onClick={() => reapply.mutate()}>
              <RefreshCw />
              {t("integrity.reapply")}
            </Button>
          </div>
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
}
