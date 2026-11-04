<script setup lang="ts">
import { computed, ref, watch, nextTick } from "vue";
import { useRoute } from "vue-router";
import { useSearchStore, matchRouteCommand, type SearchResult } from "@/stores/search";
import { encodeRouteTarget } from "@/composables/useResultActions";
import { usePluginSearchStore } from "@/stores/pluginSearch";
import { useCenterSearchStore } from "@/stores/centerSearch";
import { useToolCatalogStore } from "@/stores/toolCatalog";

const store = useSearchStore();
const route = useRoute();
const pluginSearch = usePluginSearchStore();
const centerSearch = useCenterSearchStore();
const catalog = useToolCatalogStore();
const inCenter = computed(() => route.path === "/plugins");
const centerStore = computed(() => route.query.section === "store" || !route.query.section);
const shared = computed(() => route.name === "plugin" && pluginSearch.enabled);
const text = computed({get:()=>inCenter.value ? (centerStore.value ? catalog.query : centerSearch.query) : shared.value ? pluginSearch.query : store.query,set:(value:string)=>{if(inCenter.value){if(centerStore.value)catalog.query=value;else centerSearch.query=value;}else if(shared.value)pluginSearch.query=value;else store.query=value;}});
const inputRef = ref<HTMLInputElement | null>(null);

const emit = defineEmits<{
  select: [item: SearchResult];
  back: [];
}>();

const placeholder = computed(() => {
  if (shared.value) return pluginSearch.placeholder;
  if (route.name === "plugin") return "输入到插件…";
  if (route.path.startsWith("/plugins")) return "过滤插件市场（名称、描述、标签）…";
  if (route.path.startsWith("/tool")) return "搜索工具…";
  return "搜索应用、工具或命令…";
});

watch(
  () => store.hasResults,
  () => {
    nextTick(() => {
      inputRef.value?.focus();
    });
  },
  { immediate: true },
);

async function onKeydown(e: KeyboardEvent) {
  if (e.isComposing || e.keyCode === 229) return;
  if (inCenter.value) {
    if (e.key === "Backspace" && !text.value) { e.preventDefault(); emit("back"); }
    return;
  }
  if (shared.value) {
    if (["ArrowDown", "ArrowUp", "Enter", "Escape"].includes(e.key)) {
      e.preventDefault(); pluginSearch.key = ""; await nextTick(); pluginSearch.key = e.key;
    } else if (e.key === "Backspace" && !pluginSearch.query) { e.preventDefault(); emit("back"); }
    return;
  }
  switch (e.key) {
    case "Backspace":
      if (
        store.query === "" &&
        (route.name === "plugin" || route.path.startsWith("/plugins"))
      ) {
        e.preventDefault();
        emit("back");
      }
      break;
    case "ArrowDown":
      e.preventDefault();
      store.move(1);
      break;
    case "ArrowUp":
      e.preventDefault();
      store.move(-1);
      break;
    case "Enter": {
      const cmd = matchRouteCommand(store.query);
      if (cmd) {
        e.preventDefault();
        emit("select", {
          kind: "route",
          id: encodeRouteTarget(cmd.path, cmd.query),
          title: store.query,
          subtitle: "路由命令",
          score: 1,
        });
        store.reset();
        return;
      }
      const q = store.query;
      if (!q.trim() || e.repeat) return;
      e.preventDefault();
      await store.refreshNow();
      if (store.query !== q) return;
      if (store.selected) emit("select", store.selected);
      break;
    }
  }
}

defineExpose({ focus: () => inputRef.value?.focus() });
</script>

<template>
  <label class="search-field">
    <svg class="search-field__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6.5" /><path d="m16 16 4 4" stroke-linecap="round" /></svg>
  <input
    ref="inputRef"
    v-model="text"
    class="search-box"
    type="text"
    aria-label="搜索应用、工具或命令"
    spellcheck="false"
    autocomplete="off"
    :placeholder="placeholder"
    @keydown="onKeydown"
  />
    <button v-if="text" type="button" class="search-field__clear" aria-label="清除搜索" @click="text = ''; inputRef?.focus()">×</button>
    <kbd v-else class="search-field__key" aria-hidden="true">↵</kbd>
  </label>
</template>

<style scoped>
.search-field {
  display: flex;
  align-items: center;
  flex: 1;
  gap: 10px;
  min-width: 0;
  min-height: 40px;
  padding: 0 11px;
  background: var(--surface-bg);
  border: 1px solid var(--panel-control-border);
  border-radius: 10px;
}
.search-field:focus-within { border-color: var(--accent); }
.search-field__icon { width: 18px; height: 18px; color: var(--text-dim-strong); flex: none; }
.search-field__key { font: inherit; font-size: 12px; color: var(--text-dim-strong); }
.search-field__clear { padding: 0 3px; border: 0; background: transparent; color: var(--text-dim); font-size: 18px; cursor: pointer; }

.search-box {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: inherit;
  font: inherit;
  font-size: 14px;
  height: 36px;
  caret-color: var(--accent);
  min-width: 0;
}

.search-box:focus-visible { outline: none; }

.search-box::placeholder {
  color: var(--text-dim-strong);
}
</style>
