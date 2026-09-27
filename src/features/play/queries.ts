import { useMutation } from "@tanstack/react-query";
import { notify } from "@/components/ui/toast";
import { playApi } from "./api";

export function useLaunch() {
  return useMutation({ mutationFn: playApi.launch, onError: notify.error });
}
