import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { notify } from "@/components/ui/toast";
import type { Slot } from "@/features/items/api";
import { type Preset, presetsApi } from "./api";

export const presetsKeys = {
  all: ["presets"] as const,
};

export function usePresets() {
  return useQuery({ queryKey: presetsKeys.all, queryFn: presetsApi.list });
}

/**
 * Last applied preset. `deps` are the update stamps of the swap, palette,
 * decal and map queries: any change re-checks whether it still matches.
 */
export function useActivePreset(deps: readonly number[]) {
  return useQuery({ queryKey: [...presetsKeys.all, "active", ...deps], queryFn: presetsApi.active });
}

function useInvalidate() {
  const qc = useQueryClient();
  return () => void qc.invalidateQueries({ queryKey: presetsKeys.all });
}

export function useSavePreset() {
  const invalidate = useInvalidate();
  const { t } = useTranslation("presets");
  return useMutation({
    mutationFn: (preset: Preset) => presetsApi.save(preset),
    onSuccess: () => {
      invalidate();
      notify.success(t("toast.saved"));
    },
    onError: notify.error,
  });
}

export function useDeletePreset() {
  const invalidate = useInvalidate();
  return useMutation({
    mutationFn: (id: string) => presetsApi.remove(id),
    onSuccess: invalidate,
    onError: notify.error,
  });
}

export function useCapturePreset() {
  const invalidate = useInvalidate();
  const { t } = useTranslation("presets");
  return useMutation({
    mutationFn: (name: string) => presetsApi.capture(name),
    onSuccess: () => {
      invalidate();
      notify.success(t("toast.captured"));
    },
    onError: notify.error,
  });
}

export function useUpdatePresetFromCurrent() {
  const invalidate = useInvalidate();
  const { t } = useTranslation("presets");
  return useMutation({
    mutationFn: (id: string) => presetsApi.updateFromCurrent(id),
    onSuccess: (p) => {
      invalidate();
      notify.success(t("toast.updated", { name: p.name }));
    },
    onError: notify.error,
  });
}

export function useApplyPreset() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => presetsApi.apply(id),
    // A preset touches swaps, palette, decals and maps: refresh everything.
    onSettled: () => void qc.invalidateQueries(),
    onError: notify.error,
  });
}

export function useImportPreset() {
  const invalidate = useInvalidate();
  const { t } = useTranslation("presets");
  return useMutation({
    mutationFn: (code: string) => presetsApi.importCode(code),
    onSuccess: (p) => {
      invalidate();
      notify.success(t("toast.imported", { name: p.name }));
    },
    onError: notify.error,
  });
}

export function useRandomPreset() {
  return useMutation({ mutationFn: (slots: Slot[]) => presetsApi.random(slots), onError: notify.error });
}

export function useShareCode(id: string | null) {
  return useQuery({
    queryKey: ["presets", "code", id],
    queryFn: () => presetsApi.exportCode(id as string),
    enabled: Boolean(id),
  });
}
