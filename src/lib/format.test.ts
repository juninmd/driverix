import { describe, expect, test } from "bun:test";
import { ageYears, formatBytes, formatDate, hresultHex, isOldDriver, problemText } from "./format";

describe("hresultHex", () => {
  test("shows the unsigned form Windows documents", () => {
    // E_ACCESSDENIED arrives from the backend as a negative i32.
    expect(hresultHex(-2147024891)).toBe("0x80070005");
    expect(hresultHex(0)).toBe("0x00000000");
  });
});

describe("formatBytes", () => {
  test("scales and rounds", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(250 * 1024 * 1024)).toBe("250 MB");
  });
  test("never prints 1024 of a unit; the next unit is shown instead", () => {
    // 1023.7 KB rounds to 1024 KB, which should read as 1.0 MB.
    expect(formatBytes(1048269)).toBe("1.0 MB");
    expect(formatBytes(1023.96 * 1024 * 1024)).toBe("1.0 GB");
    expect(formatBytes(1023)).toBe("1023 B");
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
    // 39 is the one that calls for a driver reinstall; without a name the user has no next step.
    expect(problemText(39)).toBe("Driver corrompido ou ausente");
    expect(problemText(99)).toBe("Problema (código 99)");
  });
});

describe("ageYears", () => {
  const now = new Date("2026-10-19T12:00:00Z");
  test("counts whole years, not crossing the anniversary early", () => {
    expect(ageYears("2023-10-19", now)).toBe(3);
    expect(ageYears("2023-10-20", now)).toBe(2);
    expect(ageYears("2026-12-01", now)).toBe(0);
  });
  test("is null for unknown dates", () => {
    expect(ageYears(null, now)).toBeNull();
    expect(ageYears("19/10/2023", now)).toBeNull();
  });
});

describe("isOldDriver", () => {
  const now = new Date("2026-10-19T12:00:00Z");
  test("flags third-party drivers from three years back", () => {
    expect(isOldDriver("2022-01-01", "Realtek", now)).toBe(true);
    expect(isOldDriver("2024-01-01", "Realtek", now)).toBe(false);
  });
  test("never flags Microsoft inbox drivers or unknown dates", () => {
    expect(isOldDriver("2006-06-21", "Microsoft", now)).toBe(false);
    expect(isOldDriver(null, "Realtek", now)).toBe(false);
  });
});
