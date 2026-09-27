import { useQuery } from "@tanstack/react-query";
import { extrasApi } from "./api";

export const extrasKeys = {
  replays: ["extras", "replays"] as const,
  bakkes: ["extras", "bakkes"] as const,
};

export function useReplays() {
  return useQuery({ queryKey: extrasKeys.replays, queryFn: extrasApi.replays, staleTime: 60_000 });
}

export function useBakkesStatus() {
  return useQuery({ queryKey: extrasKeys.bakkes, queryFn: extrasApi.bakkes, staleTime: 60_000 });
}
