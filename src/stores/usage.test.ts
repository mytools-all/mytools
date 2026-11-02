import { describe, it, expect, vi, beforeEach } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useUsageStore } from "@/stores/usage";
import { useSettingsStore } from "@/stores/settings";

const invokeMock = vi.fn(
  async (_cmd: string, _args?: unknown): Promise<unknown> => undefined,
);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...(args as [string, unknown])),
}));

describe("usage store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
  });

  it("starts empty", () => {
    const s = useUsageStore();
    expect(s.pinned).toEqual([]);
    expect(s.recent).toEqual([]);
  });

  it("reads the initial lists from the settings store", () => {
    const settings = useSettingsStore();
    settings.pinnedPlugins = ["p1"];
    settings.recentPlugins = ["p2"];
    const s = useUsageStore();
    expect(s.pinned).toEqual(["p1"]);
    expect(s.recent).toEqual(["p2"]);
  });

  it("pins and unpins", () => {
    const s = useUsageStore();
    s.pin("a");
    s.pin("b");
    expect(s.pinned).toEqual(["b", "a"]);
    s.unpin("a");
    expect(s.pinned).toEqual(["b"]);
    expect(s.isPinned("b")).toBe(true);
  });

  it("does not duplicate pins", () => {
    const s = useUsageStore();
    s.pin("a");
    s.pin("a");
    expect(s.pinned).toEqual(["a"]);
  });

  it("togglePinned flips state and returns new state", () => {
    const s = useUsageStore();
    expect(s.togglePinned("a")).toBe(true);
    expect(s.isPinned("a")).toBe(true);
    expect(s.togglePinned("a")).toBe(false);
    expect(s.isPinned("a")).toBe(false);
  });

  it("bump moves an entry to the front and dedupes", () => {
    const s = useUsageStore();
    s.bump("a");
    s.bump("b");
    s.bump("c");
    s.bump("a");
    expect(s.recent).toEqual(["a", "c", "b"]);
  });

  it("recent is capped at 12 (LRU)", () => {
    const s = useUsageStore();
    for (let i = 0; i < 20; i++) s.bump(`p${i}`);
    expect(s.recent).toHaveLength(12);
    expect(s.recent[0]).toBe("p19");
    expect(s.recent).not.toContain("p0");
  });

  it("persists through settings_set_usage in Tauri", async () => {
    (window as unknown as { __TAURI_INTERNALS__: object }).__TAURI_INTERNALS__ =
      {};
    const s = useUsageStore();
    s.pin("x");
    s.bump("y");
    await Promise.resolve();
    expect(invokeMock).toHaveBeenCalledWith("settings_set_usage", {
      pinned: ["x"],
      recent: ["y"],
    });
    const settings = useSettingsStore();
    expect(settings.pinnedPlugins).toEqual(["x"]);
    expect(settings.recentPlugins).toEqual(["y"]);
  });

  it("normalizes app separator keys", () => {
    const s = useUsageStore();
    s.togglePinnedApp("C:/Tools/Editor.exe");
    expect(s.pinned).toEqual(["app:C:\\Tools\\Editor.exe"]);
    expect(s.isPinnedApp("C:/Tools/Editor.exe")).toBe(true);
  });
});
