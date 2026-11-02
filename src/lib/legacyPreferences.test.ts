import { describe, it, expect, beforeEach } from "vitest";
import {
  clearLegacyPreferences,
  readLegacyPreferences,
} from "@/lib/legacyPreferences";

describe("legacy preference migration", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("returns null when no legacy keys exist", () => {
    expect(readLegacyPreferences()).toBeNull();
  });

  it("reads theme, hotkey and usage lists", () => {
    localStorage.setItem("mytools:theme", "dark");
    localStorage.setItem("mytools:hotkey", "Ctrl+Shift+1");
    localStorage.setItem("mytools:pinned", JSON.stringify(["p1"]));
    localStorage.setItem("mytools:recent", JSON.stringify(["p2", 1]));
    localStorage.setItem("mytools:trayEnabled", "false");

    const legacy = readLegacyPreferences();
    expect(legacy?.theme).toBe("dark");
    expect(legacy?.hotkey).toBe("Ctrl+Shift+1");
    expect(legacy?.pinned).toEqual(["p1"]);
    expect(legacy?.recent).toEqual(["p2"]);
    expect(legacy?.keys).toContain("mytools:trayEnabled");
  });

  it("ignores invalid theme and corrupted lists", () => {
    localStorage.setItem("mytools:theme", "neon");
    localStorage.setItem("mytools:pinned", "not json");
    localStorage.setItem("mytools:recent", "{ bad");
    const legacy = readLegacyPreferences();
    expect(legacy?.theme).toBeUndefined();
    expect(legacy?.pinned).toBeUndefined();
    expect(legacy?.recent).toBeUndefined();
  });

  it("clears the host mytools keys", () => {
    localStorage.setItem("mytools:theme", "dark");
    localStorage.setItem("mytools:other", "x");
    localStorage.setItem("unrelated", "keep");
    clearLegacyPreferences(["mytools:theme"]);
    expect(localStorage.getItem("mytools:theme")).toBeNull();
    expect(localStorage.getItem("mytools:other")).toBeNull();
    expect(localStorage.getItem("unrelated")).toBe("keep");
  });
});
