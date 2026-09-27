import { create } from "zustand";
import { persist } from "zustand/middleware";

interface UiState {
  sidebarCollapsed: boolean;
  commandOpen: boolean;
  /** Right-hand "active mods" panel (reset per page, see AppShell). */
  modsPanelOpen: boolean;
  /** Active swap the Items page should open for editing (then clears it). */
  swapToEdit: string | null;
  /** Preset the Presets page should open in its editor (then clears it). */
  presetToEdit: string | null;
  /** Bumped by "Check for updates" (Settings); the update dialog listens. */
  updateCheck: number;
  toggleSidebar: () => void;
  setCommandOpen: (open: boolean) => void;
  setModsPanelOpen: (open: boolean) => void;
  editSwap: (id: string | null) => void;
  editPreset: (id: string | null) => void;
  requestUpdateCheck: () => void;
}

export const useUi = create<UiState>()(
  persist(
    (set) => ({
      sidebarCollapsed: false,
      commandOpen: false,
      modsPanelOpen: false,
      swapToEdit: null,
      presetToEdit: null,
      updateCheck: 0,
      toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
      setCommandOpen: (commandOpen) => set({ commandOpen }),
      setModsPanelOpen: (modsPanelOpen) => set({ modsPanelOpen }),
      editSwap: (swapToEdit) => set({ swapToEdit }),
      editPreset: (presetToEdit) => set({ presetToEdit }),
      requestUpdateCheck: () => set((s) => ({ updateCheck: s.updateCheck + 1 })),
    }),
    { name: "alxs-ui", partialize: (s) => ({ sidebarCollapsed: s.sidebarCollapsed }) },
  ),
);
