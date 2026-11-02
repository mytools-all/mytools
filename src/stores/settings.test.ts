import { describe, it, expect, vi, beforeEach } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import {
  useSettingsStore,
  applyDocumentTheme,
  resolveTheme,
} from "@/stores/settings";

const invokeMock = vi.fn(
  async (_cmd: string, _args?: unknown): Promise<unknown> => undefined,
);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...(args as [string, unknown])),
}));

function enableTauri() {
  (window as unknown as { __TAURI_INTERNALS__: object }).__TAURI_INTERNALS__ =
    {};
}

function backendSettings(overrides: Record<string, unknown> = {}) {
  return {
    theme: "system",
    hotkey: "Alt+Space",
    trayEnabled: true,
    floatballEnabled: false,
    pinnedPlugins: [],
    recentPlugins: [],
    ...overrides,
  };
}

describe("settings store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
    document.documentElement.removeAttribute("data-theme");
    document.documentElement.style.colorScheme = "";
  });

  it("uses in-memory defaults outside Tauri", async () => {
    const s = useSettingsStore();
    await s.init();
    expect(s.theme).toBe("system");
    expect(s.hotkey).toBe("Alt+Space");
    expect(s.trayEnabled).toBe(true);
    expect(s.floatballEnabled).toBe(false);
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("loads settings from settings_get in Tauri", async () => {
    enableTauri();
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "settings_get") {
        return backendSettings({
          theme: "dark",
          hotkey: "Ctrl+Shift+1",
          trayEnabled: false,
          floatballEnabled: true,
          pinnedPlugins: ["p1"],
          recentPlugins: ["p2"],
        });
      }
      if (cmd === "hotkey_is_registered") return true;
      return undefined;
    });
    const s = useSettingsStore();
    await s.init();
    expect(s.theme).toBe("dark");
    expect(s.hotkey).toBe("Ctrl+Shift+1");
    expect(s.trayEnabled).toBe(false);
    expect(s.floatballEnabled).toBe(true);
    expect(s.pinnedPlugins).toEqual(["p1"]);
    expect(s.recentPlugins).toEqual(["p2"]);
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(s.registered).toBe(true);
  });

  it("setTheme applies immediately and persists through settings_set_theme", async () => {
    enableTauri();
    const s = useSettingsStore();
    await s.setTheme("light");
    expect(s.theme).toBe("light");
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(invokeMock).toHaveBeenCalledWith("settings_set_theme", {
      theme: "light",
    });
  });

  it("resolveTheme uses OS when system", () => {
    expect(resolveTheme("dark")).toBe("dark");
    expect(resolveTheme("light")).toBe("light");
    expect(resolveTheme("system")).toMatch(/^light|dark$/);
  });

  it("applyDocumentTheme sets dataset and colorScheme", () => {
    applyDocumentTheme("light");
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(document.documentElement.style.colorScheme).toBe("light");
    applyDocumentTheme("system");
    expect(document.documentElement.style.colorScheme).toBe("");
  });

  it("registerHotkey keeps the combo in memory outside Tauri", async () => {
    const s = useSettingsStore();
    await s.registerHotkey("Ctrl+Shift+1");
    expect(s.hotkey).toBe("Ctrl+Shift+1");
    expect(s.message).toContain("非 Tauri");
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("registerHotkey rejects empty strings", async () => {
    const s = useSettingsStore();
    await s.registerHotkey("   ");
    expect(s.message).toContain("不能为空");
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("calls hotkey_register when Tauri is available", async () => {
    enableTauri();
    const s = useSettingsStore();
    await s.registerHotkey("Super+E");
    expect(invokeMock).toHaveBeenCalledWith("hotkey_register", {
      combo: "Super+E",
    });
    expect(s.registered).toBe(true);
    expect(s.message).toBeNull();
  });

  it("unregisterHotkey sets registered false", async () => {
    enableTauri();
    const s = useSettingsStore();
    await s.unregisterHotkey();
    expect(invokeMock).toHaveBeenCalledWith("hotkey_unregister");
    expect(s.registered).toBe(false);
  });

  it("setTrayEnabled and setFloatballEnabled call their backend commands", async () => {
    enableTauri();
    const s = useSettingsStore();
    await s.setTrayEnabled(false);
    await s.setFloatballEnabled(true);
    expect(s.trayEnabled).toBe(false);
    expect(s.floatballEnabled).toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("plugin_set_tray_enabled", {
      on: false,
    });
    expect(invokeMock).toHaveBeenCalledWith("plugin_set_floatball_enabled", {
      on: true,
    });
  });

  it("setUsage calls settings_set_usage", async () => {
    enableTauri();
    const s = useSettingsStore();
    await s.setUsage(["p1"], ["p2"]);
    expect(s.pinnedPlugins).toEqual(["p1"]);
    expect(s.recentPlugins).toEqual(["p2"]);
    expect(invokeMock).toHaveBeenCalledWith("settings_set_usage", {
      pinned: ["p1"],
      recent: ["p2"],
    });
  });
});
