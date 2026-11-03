import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { tauriAvailable } from "@/lib/tauri";

export interface PluginFeature {
  code: string;
  explain: string;
  cmds: string[];
  icon?: string;
}

export interface PluginManifest {
  name: string;
  pluginName?: string;
  description: string;
  author?: string;
  version: string;
  main: string;
  logo?: string;
  icon?: string;
  preload?: string;
  homePage?: string;
  pluginType?: string;
  hostSearch?: string;
  features: PluginFeature[];
  /** Minimum host API version the plugin needs. */
  apiVersion?: number;
  /** Declared source hint; the host decides the effective origin. */
  origin?: string;
  /** Capabilities the plugin declares it needs. */
  capabilities?: string[];
  /** Explicit elevation requests, e.g. `{ exec: true }`. */
  permissions?: Record<string, unknown>;
}

/** Effective origin decided by the host. */
export type PluginOrigin = "official" | "ecosystem" | "custom";

export interface LoadedPlugin {
  manifest: PluginManifest;
  dir: string;
  mainUrl: string;
  preloadUrl?: string;
  icon?: string;
  enabled: boolean;
  origin: PluginOrigin;
  declaredCapabilities: string[];
  /** Why the plugin cannot run on this host, if it cannot. */
  incompatible?: string;
}

export const usePluginsStore = defineStore("plugins", () => {
  const plugins = ref<LoadedPlugin[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const root = ref("");
  const rootBusy = ref(false);
  const loaded = ref(false);
  let inflight: Promise<void> | null = null;
  let refreshAgain = false;
  let unlisten: UnlistenFn | null = null;

  async function loadSnapshot() {
    if (!tauriAvailable()) {
      plugins.value = [];
      loaded.value = true;
      error.value = null;
      loading.value = false;
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      plugins.value = await invoke<LoadedPlugin[]>("plugin_list");
      loaded.value = true;
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function refresh(): Promise<void> {
    if (inflight) {
      refreshAgain = true;
      return inflight;
    }
    inflight = (async () => {
      do {
        refreshAgain = false;
        await loadSnapshot();
      } while (refreshAgain);
    })();
    try { await inflight; } finally { inflight = null; }
  }

  async function ensureLoaded(): Promise<void> {
    if (loaded.value) return;
    return inflight ?? refresh();
  }

  async function reload() {
    if (!tauriAvailable()) return 0;
    try {
      const n = await invoke<number>("plugin_reload", { root: null });
      await refresh();
      return n;
    } catch (e) {
      error.value = String(e);
      return 0;
    }
  }

  async function setEnabled(code: string, enabled: boolean) {
    if (!tauriAvailable()) return;
    try {
      await invoke(enabled ? "plugin_enable" : "plugin_disable", { code });
      await refresh();
    } catch (e) {
      error.value = String(e);
    }
  }

  async function uninstall(code: string) {
    if (!tauriAvailable()) return false;
    error.value = null;
    try {
      await invoke("plugin_uninstall", { code });
      plugins.value = plugins.value.filter(plugin => plugin.manifest.name !== code);
      await refresh();
      return true;
    } catch (e) {
      error.value = String(e);
      return false;
    }
  }

  async function installDir(dir: string) {
    if (!tauriAvailable()) return "";
    try {
      const name = await invoke<string>("plugin_install_dir", { dir });
      await refresh();
      return name;
    } catch (e) {
      error.value = String(e);
      return "";
    }
  }

  async function installPack(pack: string) {
    if (!tauriAvailable()) return "";
    try {
      const name = await invoke<string>("plugin_install_pack", { pack });
      await refresh();
      return name;
    } catch (e) {
      error.value = String(e);
      return "";
    }
  }

  async function installNpm(spec: string) {
    if (!tauriAvailable()) return "";
    try {
      const name = await invoke<string>("plugin_install_npm", { spec });
      await refresh();
      return name;
    } catch (e) {
      error.value = String(e);
      return "";
    }
  }

  async function openDir(code: string) {
    if (!tauriAvailable()) return;
    try {
      await invoke("plugin_open_dir", { code });
    } catch (e) {
      error.value = String(e);
    }
  }

  async function loadRoot() {
    if (!tauriAvailable()) {
      root.value = "";
      return "";
    }
    try {
      root.value = await invoke<string>("plugin_get_root");
    } catch (e) {
      error.value = String(e);
    }
    return root.value;
  }

  async function setRoot(path: string) {
    if (!tauriAvailable()) return "";
    rootBusy.value = true;
    error.value = null;
    try {
      root.value = await invoke<string>("plugin_set_root", { root: path });
      await refresh();
      return root.value;
    } catch (e) {
      error.value = String(e);
      return "";
    } finally {
      rootBusy.value = false;
    }
  }

  async function resetRoot() {
    if (!tauriAvailable()) return "";
    rootBusy.value = true;
    error.value = null;
    try {
      root.value = await invoke<string>("plugin_reset_root");
      await refresh();
      return root.value;
    } catch (e) {
      error.value = String(e);
      return "";
    } finally {
      rootBusy.value = false;
    }
  }

  async function openRoot() {
    if (!tauriAvailable()) return;
    try {
      await invoke("plugin_open_root");
    } catch (e) {
      error.value = String(e);
    }
  }

  async function startListening() {
    if (unlisten || !tauriAvailable()) return;
    try {
      unlisten = await listen<void>("plugin:changed", () => {
        void refresh();
      });
    } catch (e) {
      error.value = String(e);
    }
  }

  function stopListening() {
    unlisten?.();
    unlisten = null;
  }

  const featureMap = computed(() => {
    const map = new Map<string, { feature: PluginFeature; plugin: LoadedPlugin }>();
    for (const p of plugins.value) {
      for (const f of p.manifest.features) {
        map.set(`${p.manifest.name}:${f.code}`, { feature: f, plugin: p });
      }
    }
    return map;
  });

  const enabledPlugins = computed(() => plugins.value.filter((p) => p.enabled));

  function findByCode(code: string) {
    for (const p of enabledPlugins.value) {
      if (p.manifest.features.some((f) => f.code === code)) return p;
    }
    return null;
  }

  function allFeatures(): { plugin: LoadedPlugin; feature: PluginFeature }[] {
    const out: { plugin: LoadedPlugin; feature: PluginFeature }[] = [];
    for (const p of enabledPlugins.value) {
      for (const f of p.manifest.features) {
        out.push({ plugin: p, feature: f });
      }
    }
    return out;
  }

  return {
    plugins,
    loading,
    error,
    root,
    rootBusy,
    featureMap,
    enabledPlugins,
    refresh,
    ensureLoaded,
    loaded,
    reload,
    setEnabled,
    uninstall,
    installDir,
    installPack,
    installNpm,
    openDir,
    loadRoot,
    setRoot,
    resetRoot,
    openRoot,
    startListening,
    stopListening,
    findByCode,
    allFeatures,
  };
});
