import { z } from "zod";

/** Mirrors `AppError::kind()` in `src-tauri/src/base/error.rs`. */
export const errorKinds = [
  "RocketLeagueNotFound",
  "GameRunning",
  "KeysMissing",
  "Upk",
  "Unsupported",
  "FileNotFound",
  "FileLocked",
  "RatingsDisabled",
  "ArenaInUse",
  "HashMismatch",
  "NotFound",
  "Conflict",
  "InvalidInput",
  "NotAllowed",
  "Network",
  "Io",
  "Serde",
  "Internal",
  "Validation",
] as const;

export type ErrorKind = (typeof errorKinds)[number];

const wireError = z.object({ kind: z.string(), message: z.string() });

export class AppError extends Error {
  readonly kind: ErrorKind;
  constructor(kind: ErrorKind, message: string) {
    super(message);
    this.name = "AppError";
    this.kind = kind;
  }
}

/** Normalises anything thrown by `invoke` (or our own code) into `AppError`. */
export function toAppError(raw: unknown): AppError {
  if (raw instanceof AppError) return raw;
  const parsed = wireError.safeParse(raw);
  if (parsed.success) {
    const kind = (errorKinds as readonly string[]).includes(parsed.data.kind)
      ? (parsed.data.kind as ErrorKind)
      : "Internal";
    return new AppError(kind, parsed.data.message);
  }
  if (raw instanceof Error) return new AppError("Internal", raw.message);
  return new AppError("Internal", String(raw));
}
