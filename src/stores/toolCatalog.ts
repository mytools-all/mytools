import { comparePluginVersions } from "@/lib/pluginVersion";
import { useCatalogSourcesStore } from "@/stores/catalogSources";
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

export type ToolInstallState = "installed" | "updateable" | "builtin" | "installable" | "planned";

export const useToolCatalogStore = defineStore("toolCatalog", () => {
  const plugins = usePluginsStore();
  const sources = useCatalogSourcesStore();
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
  let startupPromise: Promise<void> | null = null;
  let startupChecked = false;
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
        await sources.load();
        let index: unknown;
        let parsed: ToolEntry[] | undefined;
        const failures: string[] = [];
        for (const url of new Set([sources.pluginList, sources.pluginListMirror])) {
          try {
            index = await invoke<unknown>("plugin_catalog_fetch", { url });
            parsed = parseToolCatalog(index);
            sources.activeUrl = url;
            break;
          } catch (reason) { failures.push(`${url}：${String(reason)}`); }
        }
        if (!parsed) throw Error(failures.join("；"));
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

  function installedVersion(name: string) { return plugins.plugins.find(p=>p.manifest.name===name)?.manifest.version; }
  function canUpdate(entry: ToolEntry) {
    const current=installedVersion(entry.name);
    return !!current && entry.status==="ready" && !!entry.npmSpec && comparePluginVersions(entry.version,current)===1;
  }
  const updates=computed(()=>entries.value.filter(canUpdate));
  const updatingAll=ref(false);
  const updateProgress=ref({completed:0,total:0,current:""});
  function initialize():Promise<void> {
    if(startupChecked)return startupPromise??Promise.resolve();
    startupChecked=true;
    startupPromise=(async()=>{await plugins.reload();await refresh(true);})().finally(()=>{startupPromise=null;});
    return startupPromise;
  }
  async function updateSelected(names: readonly string[]) {
    const selected=new Set(names);
    await updateEntries(updates.value.filter(entry=>selected.has(entry.name)));
  }
  async function updateAll() { await updateEntries([...updates.value]); }
  async function updateEntries(entries:ToolEntry[]) {
    if(updatingAll.value||busy.value)return;
    if(!entries.length){notice.value="没有需要更新的插件";return;}
    updatingAll.value=true;
    const failures:string[]=[];let count=0;
    updateProgress.value={completed:0,total:entries.length,current:""};
    try {
      for(const entry of entries) {
        updateProgress.value.current=entry.title;
        if(await runInstall(entry))count++;else failures.push(entry.title+"："+(error.value||"更新失败"));
        updateProgress.value.completed++;
      }
      notice.value=`已更新 ${count} 个插件${failures.length?`，失败 ${failures.length} 个：${failures.join("、")}`:""}`;
      error.value=failures.length?failures.join("；"):null;
    } finally { updatingAll.value=false;updateProgress.value.current=""; }
  }
  function stateOf(entry: ToolEntry): ToolInstallState {
    if (canUpdate(entry)) return "updateable";
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
    if(updatingAll.value)return null;
    return runInstall(entry);
  }
  async function runInstall(entry: ToolEntry): Promise<string | null> {
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
    const previous=installedVersion(entry.name);
    const updating=!!previous;
    if(updating && !canUpdate(entry)) { error.value="没有可更新的已发布版本"; return null; }
    busy.value = entry.name;
    error.value = null;
    notice.value = null;
    try {
      const name = updating
        ? await invoke<string>("plugin_update_npm", { spec:entry.npmSpec,code:entry.name,fromVersion:previous,version:entry.version })
        : await invoke<string>("plugin_install_npm", { spec: entry.npmSpec });
      await plugins.refresh();
      notice.value = `${updating?"已更新":"已安装"} ${entry.title}${updating?` 至 v${entry.version}`:""}`;
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
    installedVersion,
    canUpdate,
    updates,
    updateAll,
    updateSelected,
    updateProgress,
    initialize,
    updatingAll,
    install,
    refresh,
    loading,
    loadError,
    source,
  };
});
