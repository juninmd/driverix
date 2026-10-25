import { describe, expect, test } from "bun:test";
import type { Device } from "./api";
import { devicesToTsv, sortDevices } from "./devices";

const dev = (instanceId: string, extra: Partial<Device> = {}): Device =>
  ({
    instanceId,
    name: instanceId,
    manufacturer: "",
    vendor: "",
    hint: "",
    date: null,
    problemCode: 0,
    ...extra,
  }) as Device;

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

describe("devicesToTsv", () => {
  test("writes a header and one row per device, without breaking the columns", () => {
    const tsv = devicesToTsv([dev("PCIA", { name: "Net\tcard\n2", class: "Net", version: "1.0" })]);
    expect(tsv.split("\n")).toEqual(["Dispositivo\tClasse\tVersão\tID", "Net card 2\tNet\t1.0\tPCIA"]);
  });

  test("does not let a spreadsheet run a cell as a formula", () => {
    const row = devicesToTsv([dev("X", { name: '=HYPERLINK("x")', class: "  ", version: "+1" })]).split("\n")[1];
    expect(row).toBe("'=HYPERLINK(\"x\")\t—\t'+1\tX");
  });

  test("a whitespace-only field shows a dash, not an empty cell", () => {
    expect(devicesToTsv([dev("X", { name: "   ", class: " ", version: "" })]).split("\n")[1]).toBe("X\t—\t—\tX");
  });

  test("shows a dash for missing fields and an empty list is just the header", () => {
    expect(devicesToTsv([dev("X", { name: "", class: "", version: "" })]).split("\n")[1]).toBe("X\t—\t—\tX");
    expect(devicesToTsv([]).split("\n")).toHaveLength(1);
  });
});
