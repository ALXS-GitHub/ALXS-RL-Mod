import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { FileUp, FolderInput, Map as MapIcon } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Page, PageHeader } from "@/components/ui/layout";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { notify } from "@/components/ui/toast";
import { isTauri } from "@/lib/ipc";
import { BrowseTab } from "./components/BrowseTab";
import { LibraryTab } from "./components/LibraryTab";
import { SessionBanner } from "./components/SessionBanner";
import { useImportMaps } from "./queries";

type Tab = "library" | "browse";

export function MapsPage() {
  const { t } = useTranslation("maps");
  const [tab, setTab] = useState<Tab>("library");
  const [dragging, setDragging] = useState(false);
  const importMaps = useImportMaps();

  const runImport = (paths: string[]) => {
    if (!paths.length) return;
    importMaps.mutate(paths, {
      onSuccess: (created) => {
        setTab("library");
        if (created.length) notify.success(t("toast.imported", { count: created.length }));
        else notify.info(t("toast.nothingImported"));
      },
    });
  };

  const pickFiles = async () => {
    const picked = await open({
      multiple: true,
      title: t("import.filesTitle"),
      filters: [{ name: t("import.filter"), extensions: ["upk", "udk", "zip"] }],
    });
    if (picked) runImport(Array.isArray(picked) ? picked : [picked]);
  };

  const pickFolder = async () => {
    const picked = await open({ directory: true, multiple: false, title: t("import.folderTitle") });
    if (typeof picked === "string") runImport([picked]);
  };

  // Native drag & drop of files/folders onto the window.
  // biome-ignore lint/correctness/useExhaustiveDependencies: subscribe once, import handler reads latest state
  useEffect(() => {
    if (!isTauri) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === "enter" || p.type === "over") setDragging(true);
        else if (p.type === "leave") setDragging(false);
        else if (p.type === "drop") {
          setDragging(false);
          runImport(p.paths);
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  return (
    <Page>
      <PageHeader
        icon={MapIcon}
        eyebrow={t("eyebrow")}
        title={t("title")}
        subtitle={t("subtitle")}
        actions={
          <>
            <Button variant="secondary" onClick={() => void pickFolder()} loading={importMaps.isPending}>
              <FolderInput />
              {t("actions.importFolder")}
            </Button>
            <Button variant="primary" onClick={() => void pickFiles()} loading={importMaps.isPending}>
              <FileUp />
              {t("actions.import")}
            </Button>
          </>
        }
      />

      <SessionBanner />

      <Tabs value={tab} onValueChange={(v) => setTab(v as Tab)}>
        <TabsList>
          <TabsTrigger value="library">{t("tabs.library")}</TabsTrigger>
          <TabsTrigger value="browse">{t("tabs.browse")}</TabsTrigger>
        </TabsList>
        <TabsContent value="library">
          <LibraryTab onImport={() => void pickFiles()} />
        </TabsContent>
        <TabsContent value="browse">
          <BrowseTab onShowLibrary={() => setTab("library")} />
        </TabsContent>
      </Tabs>

      <AnimatePresence>
        {dragging ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="pointer-events-none fixed inset-0 z-50 grid place-items-center bg-black/45 backdrop-blur-sm"
          >
            <motion.div
              initial={{ scale: 0.94 }}
              animate={{ scale: 1 }}
              className="glass-strong flex flex-col items-center gap-2 rounded-xl border border-dashed border-[var(--color-accent)] px-14 py-10"
            >
              <FileUp className="size-6 text-fg-muted" />
              <p className="text-[15px] font-semibold">{t("import.dropTitle")}</p>
              <p className="text-[13px] text-fg-muted">{t("import.dropHint")}</p>
            </motion.div>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </Page>
  );
}
