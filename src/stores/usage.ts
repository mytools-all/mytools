import { defineStore } from "pinia";
import { computed } from "vue";
import { useSettingsStore } from "@/stores/settings";

const RECENT_LIMIT = 12;

function normalizeAppKey(key: string): string {
  if (!key.toLowerCase().startsWith("app:")) return key;
  const path = key.slice(4);
  if (!path.includes("/")) return key;
  return `app:${path.replace(/\//g, "\\")}`;
}

export function toAppPinKey(path: string): string {
  const trimmed = path.trim();
  const raw = trimmed.toLowerCase().startsWith("app:")
    ? trimmed.slice(4)
    : trimmed;
  return `app:${raw.replace(/\//g, "\\")}`;
}

export function isAppPinKey(key: string): boolean {
  return key.startsWith("app:");
}

export function toPluginPinKey(name: string): string {
  return name;
}

export function isPluginPinKey(key: string): boolean {
  return !key.startsWith("app:");
}

export function extractAppPathFromKey(key: string): string | null {
  if (!key.startsWith("app:")) return null;
  return key.slice(4);
}

/**
 * Plugin/app usage tracking. Persistence lives in `settings.json`
 * (`pinnedPlugins` / `recentPlugins`); the settings store owns the data.
 */
export const useUsageStore = defineStore("usage", () => {
  const settings = useSettingsStore();
  const pinned = computed(() => settings.pinnedPlugins);
  const recent = computed(() => settings.recentPlugins);

  function persist() {
    void settings.setUsage([...settings.pinnedPlugins], [...settings.recentPlugins]);
  }

  function isPinned(keyOrName: string): boolean {
    if (keyOrName.toLowerCase().startsWith("app:")) {
      const target = keyOrName.toLowerCase();
      return settings.pinnedPlugins.some(
        (key) => key.toLowerCase() === target,
      );
    }
    return settings.pinnedPlugins.includes(keyOrName);
  }

  function pin(keyOrName: string) {
    const key = normalizeAppKey(keyOrName);
    if (isPinned(key)) return;
    settings.pinnedPlugins = [key, ...settings.pinnedPlugins];
    persist();
  }

  function unpin(keyOrName: string) {
    if (keyOrName.toLowerCase().startsWith("app:")) {
      const target = keyOrName.toLowerCase();
      settings.pinnedPlugins = settings.pinnedPlugins.filter(
        (key) => key.toLowerCase() !== target,
      );
      persist();
      return;
    }
    settings.pinnedPlugins = settings.pinnedPlugins.filter(
      (key) => key !== keyOrName,
    );
    persist();
  }

  function togglePinned(keyOrName: string): boolean {
    const had = isPinned(keyOrName);
    if (had) {
      unpin(keyOrName);
    } else {
      pin(keyOrName);
    }
    return !had;
  }

  function bump(name: string) {
    settings.recentPlugins = [
      name,
      ...settings.recentPlugins.filter((item) => item !== name),
    ].slice(0, RECENT_LIMIT);
    persist();
  }

  function bumpApp(pathOrKey: string) {
    const key = pathOrKey.toLowerCase().startsWith("app:")
      ? normalizeAppKey(pathOrKey)
      : toAppPinKey(pathOrKey);
    settings.recentPlugins = [
      key,
      ...settings.recentPlugins.filter(
        (item) => item.toLowerCase() !== key.toLowerCase(),
      ),
    ].slice(0, RECENT_LIMIT);
    persist();
  }

  function togglePinnedApp(pathOrKey: string): boolean {
    const key = pathOrKey.toLowerCase().startsWith("app:")
      ? pathOrKey
      : toAppPinKey(pathOrKey);
    return togglePinned(key);
  }

  function isPinnedApp(pathOrKey: string): boolean {
    const key = pathOrKey.toLowerCase().startsWith("app:")
      ? pathOrKey
      : toAppPinKey(pathOrKey);
    const target = key.toLowerCase();
    return settings.pinnedPlugins.some(
      (item) => item.toLowerCase() === target,
    );
  }

  function removeRecent(keyOrName: string) {
    if (keyOrName.toLowerCase().startsWith("app:")) {
      const target = keyOrName.toLowerCase();
      settings.recentPlugins = settings.recentPlugins.filter(
        (item) => item.toLowerCase() !== target,
      );
    } else {
      settings.recentPlugins = settings.recentPlugins.filter(
        (item) => item !== keyOrName,
      );
    }
    persist();
  }

  function clearRecent() {
    settings.recentPlugins = [];
    persist();
  }

  return {
    pinned,
    recent,
    isPinned,
    pin,
    unpin,
    togglePinned,
    bump,
    bumpApp,
    togglePinnedApp,
    isPinnedApp,
    removeRecent,
    clearRecent,
  };
});
