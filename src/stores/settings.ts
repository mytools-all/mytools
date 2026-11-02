import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { tauriAvailable } from "@/lib/tauri";
import {
  clearLegacyPreferences,
  readLegacyPreferences,
} from "@/lib/legacyPreferences";

export type Theme = "system" | "light" | "dark";

export type SettingsCategory =
  | "appearance"
  | "plugins"
  | "data"
  | "shortcuts"
  | "about";

export const SETTINGS_CATEGORIES: ReadonlyArray<{
  key: SettingsCategory;
  label: string;
}> = [
  { key: "appearance", label: "外观" },
  { key: "plugins", label: "插件" },
  { key: "data", label: "目录授权" },
  { key: "shortcuts", label: "快捷键" },
  { key: "about", label: "关于" },
];

const DEFAULT_HOTKEY = "Alt+Space";

interface AppSettings {
  theme: Theme;
  hotkey: string;
  trayEnabled: boolean;
  floatballEnabled: boolean;
  pinnedPlugins: string[];
  recentPlugins: string[];
}

export { tauriAvailable };

export function resolveTheme(t: Theme): "light" | "dark" {
  if (t !== "system") return t;
  return matchMediaLight() ? "light" : "dark";
}

function matchMediaLight(): boolean {
  return typeof window.matchMedia === "function"
    ? window.matchMedia("(prefers-color-scheme: light)").matches
    : true;
}

export function applyDocumentTheme(t: Theme) {
  document.documentElement.dataset.theme = resolveTheme(t);
  document.documentElement.style.colorScheme = t === "system" ? "" : t;
}

export const useSettingsStore = defineStore("settings", () => {
  const theme = ref<Theme>("system");
  const hotkey = ref(DEFAULT_HOTKEY);
  const trayEnabled = ref(true);
  const floatballEnabled = ref(false);
  const pinnedPlugins = ref<string[]>([]);
  const recentPlugins = ref<string[]>([]);
  const registered = ref(false);
  const loading = ref(false);
  const saving = ref(false);
  const message = ref<string | null>(null);
  const settingsCategory = ref<SettingsCategory>("appearance");

  function applyTheme() {
    applyDocumentTheme(theme.value);
  }

  function applySettings(settings: Partial<AppSettings>) {
    if (settings.theme) theme.value = settings.theme;
    if (typeof settings.hotkey === "string" && settings.hotkey.trim()) {
      hotkey.value = settings.hotkey;
    }
    if (typeof settings.trayEnabled === "boolean") {
      trayEnabled.value = settings.trayEnabled;
    }
    if (typeof settings.floatballEnabled === "boolean") {
      floatballEnabled.value = settings.floatballEnabled;
    }
    if (Array.isArray(settings.pinnedPlugins)) {
      pinnedPlugins.value = settings.pinnedPlugins.filter(
        (item): item is string => typeof item === "string",
      );
    }
    if (Array.isArray(settings.recentPlugins)) {
      recentPlugins.value = settings.recentPlugins.filter(
        (item): item is string => typeof item === "string",
      );
    }
  }

  async function refreshRegistered() {
    if (!tauriAvailable()) {
      registered.value = false;
      return;
    }
    try {
      registered.value = await invoke<boolean>("hotkey_is_registered");
    } catch (e) {
      message.value = String(e);
    }
  }

  async function migrateLegacyPreferences(): Promise<void> {
    const legacy = readLegacyPreferences();
    if (!legacy) return;
    try {
      if (legacy.theme) {
        await invoke("settings_set_theme", { theme: legacy.theme });
      }
      if (legacy.hotkey) {
        await invoke("hotkey_register", { combo: legacy.hotkey });
      }
      if (legacy.pinned || legacy.recent) {
        await invoke("settings_set_usage", {
          pinned: legacy.pinned ?? [],
          recent: legacy.recent ?? [],
        });
      }
      clearLegacyPreferences(legacy.keys);
    } catch (e) {
      message.value = `旧设置迁移失败：${String(e)}`;
    }
  }

  async function init() {
    if (tauriAvailable()) {
      await migrateLegacyPreferences();
      loading.value = true;
      try {
        const settings = await invoke<AppSettings | null>("settings_get");
        if (settings) applySettings(settings);
      } catch (e) {
        message.value = String(e);
      } finally {
        loading.value = false;
      }
    }
    applyTheme();
    await refreshRegistered();
  }

  async function setTheme(t: Theme) {
    theme.value = t;
    applyTheme();
    if (!tauriAvailable()) return;
    try {
      await invoke("settings_set_theme", { theme: t });
    } catch (e) {
      message.value = String(e);
    }
  }

  async function registerHotkey(combo: string) {
    const trimmed = combo.trim();
    if (!trimmed) {
      message.value = "快捷键不能为空";
      return;
    }
    if (!tauriAvailable()) {
      hotkey.value = trimmed;
      message.value = "非 Tauri 环境下仅在当前会话生效";
      return;
    }
    saving.value = true;
    try {
      await invoke("hotkey_register", { combo: trimmed });
      hotkey.value = trimmed;
      registered.value = true;
      message.value = null;
    } catch (e) {
      message.value = String(e);
    } finally {
      saving.value = false;
    }
  }

  async function unregisterHotkey() {
    if (!tauriAvailable()) {
      registered.value = false;
      return;
    }
    saving.value = true;
    try {
      await invoke("hotkey_unregister");
      registered.value = false;
      message.value = null;
    } catch (e) {
      message.value = String(e);
    } finally {
      saving.value = false;
    }
  }

  async function setTrayEnabled(on: boolean) {
    trayEnabled.value = on;
    if (!tauriAvailable()) return;
    try {
      await invoke("plugin_set_tray_enabled", { on });
    } catch (e) {
      message.value = String(e);
    }
  }

  async function setFloatballEnabled(on: boolean) {
    floatballEnabled.value = on;
    if (!tauriAvailable()) return;
    try {
      await invoke("plugin_set_floatball_enabled", { on });
    } catch (e) {
      message.value = String(e);
    }
  }

  async function setUsage(pinned: string[], recent: string[]) {
    pinnedPlugins.value = pinned;
    recentPlugins.value = recent;
    if (!tauriAvailable()) return;
    try {
      await invoke("settings_set_usage", { pinned, recent });
    } catch (e) {
      message.value = String(e);
    }
  }

  return {
    theme,
    hotkey,
    registered,
    loading,
    saving,
    message,
    trayEnabled,
    floatballEnabled,
    pinnedPlugins,
    recentPlugins,
    settingsCategory,
    setTheme,
    applyTheme,
    init,
    registerHotkey,
    unregisterHotkey,
    refreshRegistered,
    setTrayEnabled,
    setFloatballEnabled,
    setUsage,
  };
});
