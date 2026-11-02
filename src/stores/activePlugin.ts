import { defineStore } from "pinia";
import { computed, ref } from "vue";

export interface ActivePlugin {
  id: string;
  name: string;
  code: string;
  title: string;
  icon?: string;
}

export const useActivePluginStore = defineStore("activePlugin", () => {
  const plugin = ref<ActivePlugin | null>(null);

  const isActive = computed(() => plugin.value !== null);

  function set(p: ActivePlugin) {
    plugin.value = p;
  }

  function clear() {
    plugin.value = null;
  }

  return {
    plugin,
    isActive,
    set,
    clear,
  };
});