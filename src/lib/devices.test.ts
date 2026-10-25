import { describe, expect, test } from "bun:test";
import type { Device } from "./api";
import { sortDevices } from "./devices";

const dev = (instanceId: string, extra: Partial<Device> = {}): Device =>
  ({ instanceId, name: instanceId, date: null, problemCode: 0, ...extra }) as Device;

const ids = (list: Device[]) => list.map((d) => d.instanceId);

describe("sortDevices", () => {
  const list = [
    dev("b", { date: "2020-01-01", problemCode: 0 }),
    dev("a", { date: null, problemCode: 10 }),
    dev("c", { date: "2018-05-06", problemCode: 0 }),
  ];

  test("keeps the inventory order by default and never mutates the input", () => {
    expect(ids(sortDevices(list, "inventory"))).toEqual(["b", "a", "c"]);
    expect(ids(list)).toEqual(["b", "a", "c"]);
  });

  test("puts devices with a problem first", () => {
    expect(ids(sortDevices(list, "problems"))).toEqual(["a", "b", "c"]);
  });

  test("sorts oldest driver first and leaves unknown dates last", () => {
    expect(ids(sortDevices(list, "oldest"))).toEqual(["c", "b", "a"]);
  });

  test("sorts by name ignoring case", () => {
    const named = [dev("x", { name: "beta" }), dev("y", { name: "Alpha" })];
    expect(ids(sortDevices(named, "name"))).toEqual(["y", "x"]);
  });

  test("handles an empty list", () => {
    expect(sortDevices([], "oldest")).toEqual([]);
  });
});
