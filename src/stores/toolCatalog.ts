import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { tauriAvailable } from "@/lib/tauri";
import { usePluginsStore } from "@/stores/plugins";
import {
  TOOL_CATALOG,
  TOOL_CATEGORIES,
  type ToolEntry,
} from "@/data/toolCatalog";

export type ToolInstallState = "installed" | "builtin" | "installable" | "planned";

export const useToolCatalogStore = defineStore("toolCatalog", () => {
  const plugins = usePluginsStore();
  const entries = ref<ToolEntry[]>(TOOL_CATALOG);
  const category = ref<string>("all");
  const query = ref("");
  const busy = ref<string | null>(null);
  const error = ref<string | null>(null);
  const notice = ref<string | null>(null);

  const categories = computed(() => TOOL_CATEGORIES);

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
        entry.title.toLowerCase().includes(q) ||
        entry.description.toLowerCase().includes(q) ||
        entry.features.some((f) => f.toLowerCase().includes(q))
      );
    });
  });

  async function install(entry: ToolEntry): Promise<string | null> {
    if (!tauriAvailable()) {
      error.value = "非桌面环境无法安装插件";
      return null;
    }
    if (entry.status !== "ready") {
      error.value = `${entry.title} 还在开发中`;
      return null;
    }
    busy.value = entry.name;
    error.value = null;
    notice.value = null;
    try {
      let name: string;
      if (entry.npmSpec) {
        name = await invoke<string>("plugin_install_npm", { spec: entry.npmSpec });
      } else {
        const picked = await open({ directory: true, multiple: false, title: `选择 ${entry.title} 的插件目录` });
        if (typeof picked !== "string") return null;
        name = await invoke<string>("plugin_install_dir", { dir: picked });
      }
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
  };
});
