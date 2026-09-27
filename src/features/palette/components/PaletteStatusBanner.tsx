import { AlertTriangle, CheckCircle2, Gamepad2, Info } from "lucide-react";
import { motion } from "motion/react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { formatRelative } from "@/lib/format";
import type { PaletteStatus } from "../api";

function Banner({
  tone,
  icon,
  children,
}: {
  tone: "info" | "warning" | "success" | "danger";
  icon: ReactNode;
  children: ReactNode;
}) {
  const tones = {
    info: "border-line bg-white/[0.03] text-fg-muted",
    warning: "border-warning/25 bg-warning/[0.06] text-warning",
    success: "border-success/25 bg-success/[0.06] text-success",
    danger: "border-danger/25 bg-danger/[0.06] text-danger",
  } as const;
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className={cn(
        "flex items-center gap-2.5 rounded-lg border px-3.5 py-2.5 text-[13px] [&_svg]:size-4 [&_svg]:shrink-0",
        tones[tone],
      )}
    >
      {icon}
      <div className="min-w-0 flex-1 text-fg">{children}</div>
    </motion.div>
  );
}

export function PaletteStatusBanner({
  status,
  gameRunning,
}: {
  status: PaletteStatus | undefined;
  gameRunning: boolean;
}) {
  const { t } = useTranslation("palette");
  if (!status) return null;
  if (status.engine === "gameMissing") {
    return (
      <Banner tone="warning" icon={<AlertTriangle />}>
        {t("status.gameMissing")}
      </Banner>
    );
  }
  if (status.engine === "unsupported") {
    return (
      <Banner tone="danger" icon={<AlertTriangle />}>
        {t("status.unsupported")} <span className="text-fg-muted">{status.reason}</span>
      </Banner>
    );
  }
  if (gameRunning) {
    return (
      <Banner tone="info" icon={<Gamepad2 />}>
        {t("status.gameRunning")}
      </Banner>
    );
  }
  if (status.active?.stale) {
    return (
      <Banner tone="warning" icon={<AlertTriangle />}>
        {t("status.stale", { name: status.active.paletteName })}
      </Banner>
    );
  }
  if (status.active) {
    return (
      <Banner tone="success" icon={<CheckCircle2 />}>
        {t("status.active", {
          name: status.active.paletteName,
          when: formatRelative(status.active.appliedAt),
        })}
      </Banner>
    );
  }
  return (
    <Banner tone="info" icon={<Info />}>
      {t("status.stockInGame")}
    </Banner>
  );
}
