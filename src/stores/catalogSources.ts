import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { tauriAvailable } from "@/lib/tauri";
import defaults from "@/config/catalog.json";

export const useCatalogSourcesStore = defineStore("catalogSources", () => {
  const pluginList = ref(defaults.pluginList);
  const pluginListMirror = ref(defaults.pluginListMirror);
  const activeUrl = ref("");
  let pending: Promise<void> | null = null;
  let loaded = false;
  async function load() {
    if (loaded || !tauriAvailable()) return;
    if (!pending) pending = (async () => {
      try {
        const config = await invoke<typeof defaults>("plugin_catalog_sources_get");
        if (typeof config?.pluginList === "string" && typeof config.pluginListMirror === "string") {
          pluginList.value = config.pluginList;
          pluginListMirror.value = config.pluginListMirror;
          loaded = true;
        }
      } finally { pending = null; }
    })();
    return pending;
  }
  async function save(primary: string, mirror: string) {
    const config = { pluginList: primary.trim(), pluginListMirror: mirror.trim() };
    for (const value of Object.values(config)) {
      const url = new URL(value);
      if (url.protocol !== "https:" || url.username || url.password) throw Error("插件仓库必须使用不含账号密码的 HTTPS 地址");
    }
    if (!tauriAvailable()) throw Error("请在桌面应用中保存仓库配置");
    await invoke("plugin_catalog_sources_set", { sources: config });
    pluginList.value = config.pluginList;
    pluginListMirror.value = config.pluginListMirror;
    activeUrl.value = "";
    loaded = true;
  }
  return { pluginList, pluginListMirror, activeUrl, load, save };
});
