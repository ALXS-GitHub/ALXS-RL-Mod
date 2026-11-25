import { useMutation } from "@tanstack/react-query";
import { notify } from "@/components/ui/toast";
import i18n from "@/lib/i18n";
import { playApi } from "./api";

export function useLaunch() {
  return useMutation({
    mutationFn: playApi.launch,
    // Epic only starts the no-EAC game from its own menu: say where to click.
    onSuccess: (r) => {
      if (r.method === "epicNoEac")
        notify.info(i18n.t("play:launch.epicNoEac.title"), i18n.t("play:launch.epicNoEac.hint"));
    },
    onError: notify.error,
  });
}
