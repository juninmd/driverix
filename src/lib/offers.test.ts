import { describe, expect, test } from "bun:test";
import type { Offer } from "./api";
import { orderOffers } from "./offers";

const offer = (updateId: string, fixesProblem: boolean): Offer =>
  ({
    candidate: { updateId },
    fixesProblem,
    decision: "newer",
  }) as unknown as Offer;

const ids = (list: Offer[]) => list.map((o) => o.candidate.updateId);

describe("orderOffers", () => {
  test("puts offers that fix a problem device first, keeping the rest in order", () => {
    const sorted = orderOffers([offer("a", false), offer("b", true), offer("c", false), offer("d", true)]);
    expect(ids(sorted)).toEqual(["b", "d", "a", "c"]);
  });

  test("returns the same order when nothing fixes a problem", () => {
    expect(ids(orderOffers([offer("a", false), offer("b", false)]))).toEqual(["a", "b"]);
  });

  test("handles an empty list", () => {
    expect(orderOffers([])).toEqual([]);
  });
});
