import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { pluginDisplayName } from "@/lib/pluginPresentation";
import { useRoute } from "vue-router";
import { usePluginsStore } from "@/stores/plugins";
import { useActivePluginStore } from "@/stores/activePlugin";
import { usePluginIframeBridge } from "@/composables/pluginBridge";
import { reportPluginError, usePluginErrors } from "@/runtime/pluginErrors";
import { emitMyToolsLifecycle, MYTOOLS_API_VERSION } from "@/composables/mytools";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { tauriAvailable } from "@/lib/tauri";
import { usePluginSearchStore } from "@/stores/pluginSearch";

export function usePluginRuntime() {
  const route = useRoute();
  const store = usePluginsStore();
  const active = useActivePluginStore();
  const pluginSearch = usePluginSearchStore();
  const errorsApi = usePluginErrors();
  const bridge = usePluginIframeBridge({
    reportError: (payload) => reportPluginError(errorsKey.value, payload),
    scope: () => pluginName.value,
  });
  const frameEl = ref<HTMLIFrameElement | null>(null);

  const pluginName = computed(() => {
    const code = route.params.id as string;
    return code || "";
  });

  const featureKey = computed(() => {
    const code = route.params.code as string;
    return code || "default";
  });

  const errorsKey = computed(() => `${pluginName.value}:${featureKey.value}`);

  const currentPlugin = computed(() => {
    const plugins = store.plugins ?? [];
    for (const p of plugins) {
      if (p?.manifest?.name === pluginName.value) return p;
    }
    return null;
  });

  const iframeSrc = ref<string | null>(null);
  const loadError = ref("");
  const frameLoaded = ref(false);
  const fileBlocked = computed(() => (iframeSrc.value ?? "").startsWith("file://"));

  const currentErrors = computed(() => errorsApi.forKey(errorsKey.value));
  function currentEnv() {
    return {
      code: featureKey.value,
      type: "text" as const,
      apiVersion: MYTOOLS_API_VERSION,
      plugin: pluginName.value,
      hostSearch: pluginSearch.enabled,
    };
  }

  function enterPlugin() {
    const env = currentEnv();
    emitMyToolsLifecycle("enter", env);
    bridge.send("enter", env);
  }

  function markActive() {
    const p = currentPlugin.value;
    if (!p) return;
    active.set({
      id: p.manifest.name,
      name: p.manifest.name,
      code: featureKey.value,
      title: pluginDisplayName(p.manifest),
      icon: p.icon,
    });
  }

  let loadGeneration = 0;
  let stopDrop: UnlistenFn | undefined;
  let disposed = false;

  async function loadPlugin() {
    const generation = ++loadGeneration;
    bridge.send("out");
    bridge.teardown();
    iframeSrc.value = null;
    loadError.value = "";
    try {
      if (!store.loaded) await store.ensureLoaded();
    } catch (error) {
      if (generation === loadGeneration) loadError.value = error instanceof Error ? error.message : String(error);
      return;
    }
    if (generation !== loadGeneration) return;
    const p = currentPlugin.value;
    if (!p || p.enabled === false || p.incompatible) {
      loadError.value = "插件不存在或已停用";
      active.clear();
      return;
    }
    frameLoaded.value = false;
    pluginSearch.bind(p.manifest.name, Boolean(p.manifest.hostSearch) && !getWindowIsDetached(), p.manifest.hostSearch);
    iframeSrc.value = p.mainUrl;
    markActive();
    await nextTick();
    if (generation !== loadGeneration) return;
    bridge.install(frameEl.value);
  }

  function onFrameLoad() {
    frameLoaded.value = true;
    bridge.send("ready", currentEnv());
    enterPlugin();
  }
  function getWindowIsDetached() { return tauriAvailable() && getCurrentWebviewWindow().label !== "main"; }
  watch(() => [pluginSearch.query, pluginSearch.key], () => {
    if (pluginSearch.owner === pluginName.value && frameLoaded.value && pluginSearch.enabled) bridge.send("search", {query:pluginSearch.query,key:pluginSearch.key});
  });

  onMounted(() => {
    void loadPlugin();
    if (tauriAvailable()) void listen("mytools:files-dropped:" + getCurrentWebviewWindow().label, () => {
      if (frameLoaded.value) bridge.send("files", {});
    }).then(stop => { if (disposed) stop(); else stopDrop = stop; }).catch(error => reportPluginError(errorsKey.value, {kind:"error", message:"无法监听文件拖放：" + String(error)}));
  });

  watch([pluginName, featureKey], ([id, code], [oldId, oldCode]) => {
    if (id !== oldId) {
      void loadPlugin();
    } else if (code !== oldCode) {
      markActive();
      enterPlugin();
    }
  });

  watch(() => {
    const plugin = currentPlugin.value;
    return plugin ? {
      enabled: plugin.enabled,
      incompatible: plugin.incompatible,
      mainUrl: plugin.mainUrl,
      version: plugin.manifest.version,
    } : null;
  }, (plugin, previous) => {
    if (!iframeSrc.value) return;
    if (!plugin || plugin.enabled === false || plugin.incompatible) {
      loadGeneration++;
      bridge.send("out");
      bridge.teardown();
      iframeSrc.value = null;
      frameLoaded.value = false;
      loadError.value = "插件不可用，请检查安装状态和 API 版本";
      active.clear();
    } else if (plugin.mainUrl !== previous?.mainUrl || plugin.version !== previous?.version) {
      void loadPlugin();
    }
  });

  onBeforeUnmount(() => {
    if (pluginSearch.owner === pluginName.value) pluginSearch.clear();
    disposed = true;
    stopDrop?.();
    loadGeneration++;
    bridge.send("out");
    bridge.teardown();
    errorsApi.clear(errorsKey.value);
    emitMyToolsLifecycle("out");
    active.clear();
  });
  return { frameEl, iframeSrc, frameLoaded, fileBlocked, currentErrors, loadError, onFrameLoad };
}
