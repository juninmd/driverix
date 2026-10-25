import { describe, expect, test } from "bun:test";
import type { Device } from "./api";
import { indexDevices, selectDevices, sortDevices } from "./devices";

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

describe("selectDevices", () => {
  const now = new Date("2026-10-25T12:00:00Z");
  const devices = [
    dev("PCIVEN_10EC", { name: "Realtek NIC", class: "Net", provider: "Realtek", date: "2019-01-01" }),
    dev("USBVID_046D", { name: "Logitech Mouse", class: "HIDClass", provider: "Logitech", problemCode: 10 }),
    dev("ACPIX", { name: "Chipset", class: "System", provider: "Microsoft", date: "2006-06-21" }),
  ];
  const all = { query: "", onlyProblems: false, onlyOld: false, cls: "" };
  const ids = (list: Device[]) => list.map((d) => d.instanceId);
  const pick = (over: Partial<typeof all>) => ids(selectDevices(indexDevices(devices, now), { ...all, ...over }));

  test("returns everything with an empty filter, in input order", () => {
    expect(pick({})).toEqual(["PCIVEN_10EC", "USBVID_046D", "ACPIX"]);
  });

  test("matches the query case-insensitively over name, vendor and id", () => {
    expect(pick({ query: "  logitech " })).toEqual(["USBVID_046D"]);
    expect(pick({ query: "ven_10ec" })).toEqual(["PCIVEN_10EC"]);
  });

  test("keeps only problem devices when asked", () => {
    expect(pick({ onlyProblems: true })).toEqual(["USBVID_046D"]);
  });

  test("keeps only third-party drivers older than three years", () => {
    // Microsoft's inbox driver is old on purpose and must not count.
    expect(pick({ onlyOld: true })).toEqual(["PCIVEN_10EC"]);
  });

  test("filters by class and combines filters", () => {
    expect(pick({ cls: "Net", query: "realtek" })).toEqual(["PCIVEN_10EC"]);
    expect(pick({ cls: "Net", onlyProblems: true })).toEqual([]);
  });

  test("the old-driver flag is computed against the given clock", () => {
    const later = indexDevices(devices, new Date("2010-01-01T00:00:00Z"));
    expect(later.find((i) => i.device.instanceId === "PCIVEN_10EC")?.old).toBe(false);
  });
});
