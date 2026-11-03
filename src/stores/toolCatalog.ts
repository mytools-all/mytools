import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { tauriAvailable } from "@/lib/tauri";
import { usePluginsStore } from "@/stores/plugins";
import {
  TOOL_CATALOG,
  parseToolCatalog,
  type ToolEntry,
} from "@/data/toolCatalog";

export type ToolInstallState = "installed" | "builtin" | "installable" | "planned";

export const useToolCatalogStore = defineStore("toolCatalog", () => {
  const plugins = usePluginsStore();
  const entries = ref<ToolEntry[]>(TOOL_CATALOG);
  const loading = ref(false);
  const loadError = ref<string | null>(null);
  const source = ref<"snapshot" | "cache" | "remote">("snapshot");
  const cacheKey = "mytools.catalog.v2";
  try {
    const cached = localStorage.getItem(cacheKey);
    if (cached) { entries.value = parseToolCatalog(JSON.parse(cached)); source.value = "cache"; }
  } catch { source.value = "snapshot"; }
  let refreshPromise: Promise<void> | null = null;
  let lastAttempt = 0;
  const category = ref<string>("all");
  const query = ref("");
  const busy = ref<string | null>(null);
  const error = ref<string | null>(null);
  const notice = ref<string | null>(null);

  const categories = computed(() => [...new Set(entries.value.map(entry => entry.category))]);

  function refresh(force = false): Promise<void> {
    if (refreshPromise) return refreshPromise;
    if (!tauriAvailable() || (!force && Date.now() - lastAttempt < 60_000)) return Promise.resolve();
    lastAttempt = Date.now();
    loading.value = true;
    loadError.value = null;
    refreshPromise = (async () => {
      try {
        const index = await invoke<unknown>("plugin_catalog_fetch");
        const parsed = parseToolCatalog(index);
        entries.value = parsed;
        source.value = "remote";
        if (category.value !== "all" && !parsed.some(entry => entry.category === category.value)) category.value = "all";
        try { localStorage.setItem(cacheKey, JSON.stringify(index)); } catch { return; }
      } catch (e) {
        loadError.value = `目录更新失败，继续使用已有目录：${String(e)}`;
      } finally {
        loading.value = false;
        refreshPromise = null;
      }
    })();
    return refreshPromise;
  }

  const installedNames = computed(
    () => new Set(plugins.plugins.map((p) => p.manifest.name)),
  );

  function stateOf(entry: ToolEntry): ToolInstallState {
    if (installedNames.value.has(entry.name)) return "installed";
    if (entry.status === "builtin") return "builtin";
    if (entry.status === "ready") return "installable";
    return "planned";
  }

  const visibleEntries = computed(() => {
    const q = query.value.trim().toLowerCase();
    return entries.value.filter((entry) => {
      if (category.value !== "all" && entry.category !== category.value) {
        return false;
      }
      if (!q) return true;
      return (
        entry.name.toLowerCase().includes(q) ||
        entry.npm.toLowerCase().includes(q) ||
        entry.keywords.some(word => word.toLowerCase().includes(q)) ||
        entry.title.toLowerCase().includes(q) ||
        entry.description.toLowerCase().includes(q) ||
        entry.features.some((f) => f.toLowerCase().includes(q))
      );
    });
  });

  async function install(entry: ToolEntry): Promise<string | null> {
    if (busy.value) return null;
    if (!tauriAvailable()) {
      error.value = "非桌面环境无法安装插件";
      return null;
    }
    if (entry.status !== "ready") {
      error.value = `${entry.title} 还在开发中`;
      return null;
    }
    if (!entry.npmSpec) {
      error.value = "索引未提供可安装的 npm 包";
      return null;
    }
    busy.value = entry.name;
    error.value = null;
    notice.value = null;
    try {
      const name = await invoke<string>("plugin_install_npm", { spec: entry.npmSpec });
      await plugins.refresh();
      notice.value = `已安装 ${entry.title}`;
      return name;
    } catch (e) {
      error.value = String(e);
      return null;
    } finally {
      busy.value = null;
    }
  }

  return {
    entries,
    categories,
    category,
    query,
    busy,
    error,
    notice,
    installedNames,
    visibleEntries,
    stateOf,
    install,
    refresh,
    loading,
    loadError,
    source,
  };
});
