import { describe, expect, test } from "bun:test";
import type { Offer } from "./api";
import { sortOffers } from "./offerSort";

const offer = (updateId: string, sizeBytes: number, date: string | null): Offer =>
  ({ candidate: { updateId, sizeBytes, date }, decision: "newer" }) as unknown as Offer;

const ids = (list: Offer[]) => list.map((o) => o.candidate.updateId);

describe("sortOffers", () => {
  const list = [offer("b", 100, "2024-01-01"), offer("a", 900, null), offer("c", 500, "2025-06-01")];

  test("keeps the Windows Update order by default and never mutates the input", () => {
    expect(ids(sortOffers(list, "source"))).toEqual(["b", "a", "c"]);
    expect(ids(list)).toEqual(["b", "a", "c"]);
  });

  test("puts the largest download first", () => {
    expect(ids(sortOffers(list, "size"))).toEqual(["a", "c", "b"]);
  });

  test("puts the newest driver first and unknown dates last", () => {
    expect(ids(sortOffers(list, "date"))).toEqual(["c", "b", "a"]);
  });

  test("keeps the Windows Update order between equal sizes", () => {
    const same = [offer("x", 10, null), offer("y", 10, null)];
    expect(ids(sortOffers(same, "size"))).toEqual(["x", "y"]);
  });
});
