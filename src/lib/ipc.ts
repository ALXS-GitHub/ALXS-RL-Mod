import { invoke } from "@tauri-apps/api/core";
import type { z } from "zod";
import { AppError, toAppError } from "./errors";

/** True when running inside the Tauri webview (false in a plain browser tab). */
export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * Typed IPC call. Every response is validated with a zod schema so a
 * backend/front-end drift fails loudly at the boundary instead of deep in
 * a component.
 *
 * @example
 *   const status = await call("game_status", {}, gameStatusSchema);
 */
export async function call<S extends z.ZodType>(
  command: string,
  args: Record<string, unknown>,
  schema: S,
): Promise<z.infer<S>> {
  let raw: unknown;
  try {
    raw = await invoke(command, args);
  } catch (err) {
    throw toAppError(err);
  }
  const parsed = schema.safeParse(raw);
  if (!parsed.success) {
    console.error(`[ipc] ${command}: unexpected response`, parsed.error.issues, raw);
    throw new AppError("Validation", `Unexpected response from ${command}`);
  }
  return parsed.data;
}

/** For commands that return nothing (`()` on the Rust side). */
export async function callVoid(command: string, args: Record<string, unknown> = {}): Promise<void> {
  try {
    await invoke(command, args);
  } catch (err) {
    throw toAppError(err);
  }
}
