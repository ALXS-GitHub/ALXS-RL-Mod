import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import type { z } from "zod";
import { isTauri } from "./ipc";

/** Event names emitted by the backend (see docs/ARCHITECTURE.md). */
export const EVENTS = {
  gameStatus: "game://status",
  integrityReport: "integrity://report",
  mapsDownload: "maps://download",
  statsEvent: "stats://event",
  trackerSession: "tracker://session",
} as const;

/**
 * Subscribes to a backend event for the lifetime of the component. Payloads
 * are validated; invalid payloads are logged and dropped.
 */
export function useTauriEvent<S extends z.ZodType>(
  event: string,
  schema: S,
  handler: (payload: z.infer<S>) => void,
) {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    if (!isTauri) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listen<unknown>(event, (e) => {
      const parsed = schema.safeParse(e.payload);
      if (parsed.success) handlerRef.current(parsed.data);
      else console.warn(`[events] ${event}: invalid payload`, parsed.error.issues);
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [event, schema]);
}
