import { defineStore } from "pinia";
import { useLocalStorage } from "@vueuse/core";
import type { LoadedPlugin } from "@/stores/plugins";
import { pluginDisplayName } from "@/lib/pluginPresentation";
import { validPanelEntry, type PanelEntry } from "@/lib/panelEntries";

export const usePanelStore = defineStore("panel", () => {
  const entries = useLocalStorage<PanelEntry[]>("mytools-panel-entries-v1", []);
  function hasPlugin(plugin: string, feature?: string) {
    return Array.isArray(entries.value) && entries.value.some(e => validPanelEntry(e) && e.kind === "plugin" && e.target === plugin && (!feature || e.feature === feature));
  }
  function addPlugin(plugin: LoadedPlugin, feature = plugin.manifest.features[0]?.code) {
    if (!feature || hasPlugin(plugin.manifest.name, feature)) return;
    const entry: PanelEntry = { id: crypto.randomUUID(), kind: "plugin", target: plugin.manifest.name, feature, title: pluginDisplayName(plugin.manifest) };
    entries.value = [...(Array.isArray(entries.value) ? entries.value.filter(validPanelEntry) : []), entry];
  }
  return { entries, hasPlugin, addPlugin };
});
