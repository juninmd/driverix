import { describe, expect, test } from "bun:test";
import type { HardwareView } from "../lib/api";
import { lastView } from "./useHardware";

const view = { sections: [], report: "r" } as unknown as HardwareView;

describe("lastView", () => {
  test("keeps the view through a failed refresh and the retry after it", () => {
    // done -> refresh -> fails -> retry: the panel must stay on screen the whole way.
    const failed = { kind: "error" as const, message: "x", previous: lastView({ kind: "loading", previous: view }) };
    expect(failed.previous).toBe(view);
    expect(lastView(failed)).toBe(view);
  });

  test("takes the view from a finished read", () => {
    expect(lastView({ kind: "done", view })).toBe(view);
  });

  test("has nothing before the first read", () => {
    expect(lastView({ kind: "idle" })).toBeUndefined();
    expect(lastView({ kind: "error", message: "x" })).toBeUndefined();
  });
});
