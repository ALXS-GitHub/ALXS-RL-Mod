import { describe, expect, it } from "vitest";
import { AppError, toAppError } from "./errors";

describe("toAppError", () => {
  it("keeps a known backend kind", () => {
    const e = toAppError({ kind: "GameRunning", message: "close the game" });
    expect(e).toBeInstanceOf(AppError);
    expect(e.kind).toBe("GameRunning");
    expect(e.message).toBe("close the game");
  });

  it("maps an unknown kind to Internal", () => {
    expect(toAppError({ kind: "Nope", message: "x" }).kind).toBe("Internal");
  });

  it("wraps plain errors and strings", () => {
    expect(toAppError(new Error("boom")).message).toBe("boom");
    expect(toAppError("raw").kind).toBe("Internal");
  });

  it("returns AppError instances unchanged", () => {
    const e = new AppError("Network", "offline");
    expect(toAppError(e)).toBe(e);
  });
});
