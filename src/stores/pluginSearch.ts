import { defineStore } from "pinia";
import { ref } from "vue";

export const usePluginSearchStore = defineStore("pluginSearch", () => {
  const owner = ref("");
  const enabled = ref(false);
  const query = ref("");
  const placeholder = ref("搜索插件内容…");
  const key = ref("");
  function bind(id: string, shared: boolean, hint?: string) {
    owner.value = id; enabled.value = shared; query.value = ""; key.value = "";
    placeholder.value = hint || "搜索插件内容…";
  }
  function clear() { bind("", false); }
  return { owner, enabled, query, placeholder, key, bind, clear };
});
