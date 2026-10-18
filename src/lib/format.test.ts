import { describe, expect, test } from "bun:test";
import { formatBytes, formatDate, problemText } from "./format";

describe("formatBytes", () => {
  test("scales and rounds", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(250 * 1024 * 1024)).toBe("250 MB");
  });
  test("shows a dash for unknown sizes", () => {
    expect(formatBytes(0)).toBe("—");
    expect(formatBytes(Number.NaN)).toBe("—");
  });
});

describe("formatDate", () => {
  test("reorders without timezone drift", () => {
    expect(formatDate("2024-05-06")).toBe("06/05/2024");
  });
  test("rejects anything that is not YYYY-MM-DD", () => {
    expect(formatDate(null)).toBe("—");
    expect(formatDate("2024-05-06T00:00:00Z")).toBe("—");
  });
});

describe("problemText", () => {
  test("names the common codes and falls back to the number", () => {
    expect(problemText(0)).toBe("OK");
    expect(problemText(28)).toBe("Sem driver instalado");
    expect(problemText(99)).toBe("Problema (código 99)");
  });
});
