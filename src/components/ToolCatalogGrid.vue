<script setup lang="ts">
import { computed, onMounted } from "vue";
import TopTabNav from "@/components/TopTabNav.vue";
import { useToolCatalogStore } from "@/stores/toolCatalog";
import { fallbackIconDataUri } from "@/lib/iconFallback";
import { usePluginsStore } from "@/stores/plugins";
import type { ToolEntry } from "@/data/toolCatalog";

const store = useToolCatalogStore();
const plugins = usePluginsStore();
withDefaults(defineProps<{sharedSearch?: boolean}>(), {sharedSearch:false});
const emit = defineEmits<{detail:[name:string]}>();
onMounted(() => { void store.refresh(); });

const tabs = computed(() => [
  { id: "all", label: "全部", count: store.visibleEntries.length },
  ...store.categories.map((c) => ({
    id: c,
    label: c,
    count: store.entries.filter((e) => e.category === c).length,
  })),
]);

function statusLabel(entry: ToolEntry): string {
  const state = store.stateOf(entry);
  if (state === "installed") return "已安装";
  if (state === "builtin") return "已内置";
  if (state === "installable") return "可安装";
  return "开发中";
}

function statusClass(entry: ToolEntry): string {
  const state = store.stateOf(entry);
  if (state === "installed") return "tc__badge--ok";
  if (state === "installable") return "tc__badge--accent";
  return "tc__badge--dim";
}

function onInstall(entry: ToolEntry) {
  void store.install(entry);
}
function brief(entry: ToolEntry) { return entry.description.split(/[。！？\r\n]/)[0] || "暂无介绍"; }
</script>

<template>
  <div class="tc">
    <div class="tc__head">
      <input
        v-if="!sharedSearch"
        v-model="store.query"
        class="tc__search"
        spellcheck="false"
        placeholder="搜索工具…"
      />
      <button class="tc__btn" :disabled="store.loading" @click="store.refresh(true)">{{ store.loading ? '更新中…' : '刷新目录' }}</button>
    </div>
    <TopTabNav v-model="store.category as unknown as string" :tabs="tabs" />
    <p v-if="store.loadError" class="tc__banner" role="status">{{ store.loadError }}</p>

    <p v-if="store.error" class="tc__banner tc__banner--err">{{ store.error }}</p>
    <p v-else-if="store.notice" class="tc__banner tc__banner--ok">
      {{ store.notice }}
    </p>

    <div v-if="store.visibleEntries.length === 0" class="tc__empty">
      没有匹配的工具。
    </div>

    <div v-else class="tc__grid">
      <article v-for="entry in store.visibleEntries" :key="entry.name" class="tc__card" role="button" tabindex="0" @click="emit('detail',entry.name)" @keydown.enter.self="emit('detail',entry.name)" @keydown.space.self.prevent="emit('detail',entry.name)">
        <header class="tc__card-head">
          <img
            :src="plugins.plugins.find(p=>p.manifest.name===entry.name)?.icon || entry.icon || fallbackIconDataUri(entry.title)"
            :alt="entry.title"
            class="tc__icon"
          />
          <div class="tc__title-wrap">
            <h3 class="tc__title">{{ entry.title }}</h3>
            <p class="tc__meta">{{ entry.category }} · v{{ entry.version }}</p>
          </div>
          <span class="tc__badge" :class="statusClass(entry)">
            {{ statusLabel(entry) }}
          </span>
        </header>

        <p class="tc__desc">{{ brief(entry) }}。</p>

        <footer class="tc__foot">
          <span class="tc__ref">查看详情 →</span>
          <button
            v-if="store.stateOf(entry) === 'installable'"
            class="tc__btn tc__btn--primary"
            :disabled="!!store.busy"
            @click.stop="onInstall(entry)"
          >
            {{ store.busy === entry.name ? "安装中…" : "安装" }}
          </button>
          <span v-else-if="store.stateOf(entry) === 'installed'" class="tc__state">
            在「已安装」中打开
          </span>
          <span v-else-if="store.stateOf(entry) === 'builtin'" class="tc__state">
            随应用内置
          </span>
          <span v-else class="tc__state">开发中</span>
        </footer>
      </article>
    </div>
  </div>
</template>

<style scoped>
.tc {
  display: flex;
  flex-direction: column;
  gap: 14px;
  scrollbar-gutter: stable;
  overflow-y: auto;
  min-height: 0;
}

.tc__head {
  display: flex;
  gap: 8px;
}

.tc__search {
  flex: 1;
  padding: 8px 12px;
  border-radius: 8px;
  border: 1px solid var(--panel-control-border);
  background: transparent;
  color: var(--panel-fg);
  font-size: 13px;
  outline: none;
}

.tc__search:focus {
  border-color: var(--accent);
}

.tc__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 14px;
  align-content: start;
}

.tc__card:hover {border-color:var(--accent);}
.tc__card:focus-visible {outline:2px solid var(--accent);outline-offset:2px;}
.tc__card {
  cursor:pointer;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 16px;
  border: 1px solid var(--panel-control-border);
  border-radius: 10px;
  background: var(--surface-bg);
}

.tc__card-head {
  display: flex;
  align-items: center;
  gap: 10px;
}

.tc__icon {
  width: 32px;
  height: 32px;
  flex: none;
  border-radius: 8px;
  object-fit: contain;
  background: var(--hover-bg);
}

.tc__title-wrap {
  flex: 1;
  min-width: 0;
}

.tc__title {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--panel-fg);
}

.tc__meta {
  margin: 0;
  font-size: 11px;
  color: var(--text-dim-strong);
}

.tc__badge {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 10px;
  border: 1px solid var(--text-dim-strong);
  color: var(--text-dim-strong);
  white-space: nowrap;
}

.tc__badge--ok {
  border-color: #3fa96b;
  color: #3fa96b;
}

.tc__badge--accent {
  border-color: var(--accent);
  color: var(--accent);
}

.tc__badge--dim {
  opacity: 0.7;
}

.tc__desc {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim);
  line-height: 1.5;
}

.tc__features {
  margin: 0;
  padding-left: 16px;
  font-size: 11px;
  color: var(--text-dim-strong);
  line-height: 1.6;
}

.tc__caps {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.tc__cap {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
  background: var(--chip-bg);
  color: var(--accent);
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
}

.tc__foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-top: auto;
  padding-top: 4px;
  border-top: 1px solid var(--panel-border);
}

.tc__ref {
  font-size: 10px;
  color: var(--text-dim-strong);
  text-decoration: none;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tc__ref:hover {
  color: var(--accent);
  text-decoration: underline;
}

.tc__ref--none {
  opacity: 0.5;
}

.tc__state {
  font-size: 11px;
  color: var(--text-dim-strong);
  white-space: nowrap;
}

.tc__btn {
  padding: 4px 12px;
  border-radius: 6px;
  border: 1px solid var(--panel-control-border);
  background: transparent;
  color: var(--panel-fg);
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}

.tc__btn--primary {
  border-color: var(--accent);
  color: var(--accent);
}

.tc__btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.tc__banner {
  margin: 0;
  padding: 6px 10px;
  border-radius: 6px;
  font-size: 12px;
}

.tc__banner--err {
  background: rgba(224, 108, 117, 0.12);
  color: #e06c75;
}

.tc__banner--ok {
  background: rgba(63, 169, 107, 0.12);
  color: #3fa96b;
}

.tc__empty {
  padding: 24px;
  text-align: center;
  font-size: 12px;
  color: var(--text-dim-strong);
  border: 1px dashed var(--panel-control-border);
  border-radius: 8px;
}
</style>
