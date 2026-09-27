import { z } from "zod";

/**
 * Report produced by the integrity module (startup check, manual check,
 * re-apply). Shared here because the shell shows it as a toast while the
 * Items page renders it in detail. Mirrors `integrity::IntegrityReport`.
 */
export const integrityReportSchema = z.object({
  /** The game build changed since our files were written. */
  gameUpdated: z.boolean(),
  /** Our files that the game put back to stock ("verify files", update). */
  filesReverted: z.number(),
  /** Features that were rebuilt automatically (`swap`, `palette`, `decals`, `maps`). */
  reapplied: z.array(z.string()),
  /** Features that need the user (auto re-apply disabled or failed). */
  pending: z.array(z.string()),
  failures: z.array(z.object({ feature: z.string(), message: z.string() })),
  checkedAt: z.string(),
});
export type IntegrityReport = z.infer<typeof integrityReportSchema>;
