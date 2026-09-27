import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { FolderOpen, FolderPlus, RotateCcw, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { GlassPanel } from "@/components/ui/glass";
import { Section } from "@/components/ui/layout";
import { Tooltip } from "@/components/ui/tooltip";
import type { LibraryRoot } from "../api";

interface LibraryFoldersProps {
  roots: readonly LibraryRoot[];
  configured: readonly string[];
  busy: boolean;
  onChange: (folders: string[]) => void;
}

export function LibraryFolders({ roots, configured, busy, onChange }: LibraryFoldersProps) {
  const { t } = useTranslation("decals");

  const add = async () => {
    const picked = await open({ directory: true, multiple: false, title: t("folders.pick") });
    if (typeof picked === "string" && !configured.includes(picked)) onChange([...configured, picked]);
  };

  return (
    <Section
      title={t("folders.title")}
      description={t("folders.description")}
      actions={
        <div className="flex gap-2">
          {configured.length > 0 ? (
            <Button size="sm" variant="ghost" onClick={() => onChange([])} disabled={busy}>
              <RotateCcw />
              {t("folders.useDefault")}
            </Button>
          ) : null}
          <Button size="sm" variant="secondary" onClick={() => void add()} loading={busy}>
            <FolderPlus />
            {t("folders.add")}
          </Button>
        </div>
      }
    >
      <GlassPanel className="divide-y divide-line">
        {roots.length === 0 ? <p className="p-4 text-[13px] text-fg-muted">{t("folders.none")}</p> : null}
        {roots.map((root) => (
          <div key={root.path} className="flex items-center gap-3 px-4 py-2.5">
            <FolderOpen className="size-4 shrink-0 text-fg-subtle" />
            <div className="min-w-0 flex-1">
              <p className="truncate font-mono text-xs" data-selectable title={root.path}>
                {root.path}
              </p>
              <p className="text-[11px] text-fg-subtle">
                {root.exists ? t("folders.packs", { count: root.packCount }) : t("folders.missing")}
              </p>
            </div>
            {root.isDefault ? <Badge tone="info">{t("folders.default")}</Badge> : null}
            {!root.exists ? <Badge tone="warning">{t("folders.notFound")}</Badge> : null}
            {root.exists ? (
              <Tooltip content={t("folders.reveal")}>
                <Button
                  size="icon-sm"
                  variant="ghost"
                  aria-label={t("folders.reveal")}
                  onClick={() => void revealItemInDir(root.path)}
                >
                  <FolderOpen />
                </Button>
              </Tooltip>
            ) : null}
            {!root.isDefault ? (
              <Tooltip content={t("folders.remove")}>
                <Button
                  size="icon-sm"
                  variant="ghost"
                  className="hover:text-danger"
                  aria-label={t("folders.remove")}
                  disabled={busy}
                  onClick={() => onChange(configured.filter((f) => f !== root.path))}
                >
                  <Trash2 />
                </Button>
              </Tooltip>
            ) : null}
          </div>
        ))}
      </GlassPanel>
    </Section>
  );
}
