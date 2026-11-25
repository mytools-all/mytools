<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { useToolCatalogStore } from "@/stores/toolCatalog";
import TopTabNav from "@/components/TopTabNav.vue";
import { pluginDisplayName } from "@/lib/pluginPresentation";
import { fallbackIconDataUri } from "@/lib/iconFallback";
import { safeInvoke } from "@/lib/tauri";
import { usePluginsStore, type LoadedPlugin, type PluginOrigin } from "@/stores/plugins";
import { useUsageStore } from "@/stores/usage";
import { useCenterSearchStore } from "@/stores/centerSearch";
import { persistedUiValue } from "@/stores/uiState";

const props = defineProps<{ detached: boolean }>();
const emit = defineEmits<{ detail: [name: string] }>();
const plugins = usePluginsStore();
const catalog = useToolCatalogStore();
onMounted(()=>{void catalog.initialize();});
function updateEntry(name:string) { return catalog.updates.find(entry=>entry.name===name); }
async function checkUpdates(){await Promise.all([plugins.reload(),catalog.refresh(true)]);}
const usage = useUsageStore();
const search = useCenterSearchStore();
const router = useRouter();
const enableFilter = persistedUiValue<string>("installed.filter", "all", (value): value is string => typeof value === "string" && ["all", "enabled", "disabled", "updates"].includes(value));
const selected=ref(new Set<string>());
const updating=computed(()=>catalog.updatingAll||!!catalog.busy);
watch([enableFilter,()=>search.query],()=>{selected.value=new Set();});
watch(()=>catalog.updates.map(entry=>entry.name),names=>{const available=new Set(names);selected.value=new Set([...selected.value].filter(name=>available.has(name)));});
const pending = ref<Record<string, boolean>>({});
const actionError = ref<string | null>(null);
const pinnedIdSet = computed(() => new Set(usage.pinned));

const installTabs = computed(() => {
  const all = plugins.plugins.length;
  const enabled = plugins.enabledPlugins.length;
  return [
    { id: "all", label: "全部", count: all },
    { id: "enabled", label: "已启用", count: enabled, tone: "ok" as const },
    { id: "disabled", label: "已停用", count: all - enabled, tone: "err" as const },
    { id:"updates", label:"可更新", count:catalog.updates.length },
  ];
});
const installedPlugins = computed<LoadedPlugin[]>(() => {
  let xs = [...plugins.plugins];
  if (enableFilter.value === "enabled") {
    xs = xs.filter((p) => p.enabled);
  } else if (enableFilter.value === "disabled") {
    xs = xs.filter((p) => !p.enabled);
  }
  if(enableFilter.value==="updates")xs=xs.filter(p=>!!updateEntry(p.manifest.name));
  const q = search.query.trim().toLowerCase();
  if (q) {
    xs = xs.filter(
      (p) =>
        (p.manifest.pluginName ?? p.manifest.name).toLowerCase().includes(q) ||
        p.manifest.description.toLowerCase().includes(q) ||
        p.manifest.name.toLowerCase().includes(q),
    );
  }
  return xs;
});
const visibleUpdates=computed(()=>installedPlugins.value.filter(plugin=>!!updateEntry(plugin.manifest.name)));
const allVisibleSelected=computed(()=>visibleUpdates.value.length>0&&visibleUpdates.value.every(plugin=>selected.value.has(plugin.manifest.name)));
function selectOne(name:string,checked:boolean){const next=new Set(selected.value);if(checked)next.add(name);else next.delete(name);selected.value=next;}
function selectVisible(checked:boolean){const next=new Set(selected.value);for(const plugin of visibleUpdates.value){if(checked)next.add(plugin.manifest.name);else next.delete(plugin.manifest.name);}selected.value=next;}
async function updateSelection(){await catalog.updateSelected([...selected.value]);}

function pluginTitle(p: LoadedPlugin): string {
  return pluginDisplayName(p.manifest);
}

function originLabel(origin: PluginOrigin): string {
  if (origin === "official") return "官方";
  return "自定义";
}

function pluginPinLabel(name: string): string {
  return usage.isPinned(name) ? "取消固定" : "固定";
}

function onPin(name: string) {
  usage.togglePinned(name);
}

async function toggle(p: LoadedPlugin) {
  pending.value[p.manifest.name] = true;
  actionError.value = null;
  try {
    await plugins.setEnabled(p.manifest.name, !p.enabled);
  } catch (e) {
    actionError.value = String(e);
  } finally {
    pending.value[p.manifest.name] = false;
  }
}

async function openPlugin(p: LoadedPlugin) {
  const code = p.manifest.features?.[0]?.code ?? "default";
  usage.bump(p.manifest.name);
  if (props.detached) {
    try { await safeInvoke("plugin_open_window", {pluginId:p.manifest.name, feature:code}); }
    catch (error) { actionError.value = String(error); }
    return;
  }
  void router.push({ name: "plugin", params: { id: p.manifest.name, code } });
}

async function reveal(p: LoadedPlugin) {
  pending.value[p.manifest.name] = true;
  actionError.value = null;
  try {
    await plugins.openDir(p.manifest.name);
  } catch (e) {
    actionError.value = String(e);
  } finally {
    pending.value[p.manifest.name] = false;
  }
}

async function remove(p: LoadedPlugin) {
  pending.value[p.manifest.name] = true;
  actionError.value = null;
  try {
    await plugins.uninstall(p.manifest.name);
  } catch (e) {
    actionError.value = String(e);
  } finally {
    pending.value[p.manifest.name] = false;
  }
}
</script>

<template>
  <section class="pc__page">
    <input v-if="detached" v-model="search.query" class="pc__search-input pc__installed-search" placeholder="搜索已安装插件…" aria-label="搜索已安装插件" />
    <div class="pc__row"><button class="pc__btn" :disabled="plugins.loading || catalog.loading || updating" @click="checkUpdates">{{ catalog.loading?'检查中…':'检查更新 / 重新扫描' }}</button></div>
    <p v-if="catalog.loadError" class="pc__error">{{ catalog.loadError }}</p><p v-if="catalog.notice" role="status">{{ catalog.notice }}</p><p v-if="catalog.error" class="pc__error">{{ catalog.error }}</p>
    <TopTabNav v-model="enableFilter" :tabs="installTabs" />
    <div v-if="enableFilter==='updates'" class="pc__row pc__row--wrap update-tools">
      <label class="pc__settings-toggle"><input type="checkbox" class="pc__checkbox" :checked="allVisibleSelected" :disabled="updating || !visibleUpdates.length" @change="selectVisible(($event.target as HTMLInputElement).checked)">全选当前列表</label>
      <button class="pc__btn pc__btn--primary" :disabled="updating || !selected.size" @click="updateSelection">更新已选（{{ selected.size }}）</button>
      <button class="pc__btn" :disabled="updating || !catalog.updates.length" @click="catalog.updateAll()">更新全部（{{ catalog.updates.length }}）</button>
      <span v-if="catalog.updatingAll" role="status">正在更新 {{ catalog.updateProgress.completed }}/{{ catalog.updateProgress.total }} · {{ catalog.updateProgress.current }}</span>
    </div>
    <div v-if="plugins.plugins.length === 0 && !plugins.loading" class="pc__empty">
      暂无插件。切换到「开发者」安装工具包，或从本地目录安装。
    </div>
    <div v-else-if="installedPlugins.length === 0 && !plugins.loading" class="pc__empty">
      {{ enableFilter === 'updates' ? '当前列表没有可更新的插件。' : '没有匹配的插件。' }}
    </div>
    <ul v-else class="pc__ilist">
      <li v-for="p in installedPlugins" :key="p.manifest.name" class="pc__iitem" role="button" tabindex="0" @click="emit('detail', p.manifest.name)" @keydown.enter.self="emit('detail', p.manifest.name)" @keydown.space.self.prevent="emit('detail', p.manifest.name)">
        <input v-if="enableFilter==='updates'" type="checkbox" class="pc__checkbox" :aria-label="'选择更新 '+pluginTitle(p)" :checked="selected.has(p.manifest.name)" :disabled="updating" @click.stop @keydown.stop @change="selectOne(p.manifest.name,($event.target as HTMLInputElement).checked)">
        <span class="pc__iitem-icon">
          <img
            :src="p.icon || fallbackIconDataUri(pluginTitle(p))"
            :alt="pluginTitle(p)"
            class="pc__iitem-icon-img"
          />
        </span>
        <div class="pc__iitem-body">
          <div class="pc__iitem-title-row">
            <span class="pc__iitem-title">{{ pluginTitle(p) }}</span>
            <span class="pc__badge" :class="{ 'pc__badge--off': !p.enabled }">
              {{ p.enabled ? "启用" : "停用" }}
            </span>
            <span class="pc__badge pc__badge--origin" :class="`pc__badge--${p.origin}`">
              {{ originLabel(p.origin) }}
            </span>
            <span
              v-if="p.incompatible"
              class="pc__badge pc__badge--warn"
              :title="p.incompatible"
            >
              不兼容
            </span>
            <button
              type="button"
              class="pc__star"
              :class="{ 'pc__star--on': pinnedIdSet.has(p.manifest.name) }"
              :title="pluginPinLabel(p.manifest.name)"
              :aria-label="pluginPinLabel(p.manifest.name)"
              @click.stop="onPin(p.manifest.name)"
            >
              {{ pinnedIdSet.has(p.manifest.name) ? "★" : "☆" }}
            </button>
          </div>
          <p class="pc__iitem-desc">{{ p.manifest.description }}</p>
          <p class="pc__iitem-meta">
            v{{ p.manifest.version }}<template v-if="updateEntry(p.manifest.name)"> → v{{ updateEntry(p.manifest.name)?.version }} · 可更新</template> · {{ p.manifest.name }}
          </p>
          <p v-if="p.incompatible" class="pc__iitem-warn">{{ p.incompatible }}</p>
        </div>
        <div class="pc__iactions" @click.stop>
          <button v-if="updateEntry(p.manifest.name)" class="pc__btn pc__btn--primary" :disabled="updating || !!pending[p.manifest.name]" @click="catalog.install(updateEntry(p.manifest.name)!)">{{ catalog.busy===p.manifest.name?'更新中…':'更新' }}</button>
          <button
            class="pc__btn pc__btn--primary"
            :disabled="!p.enabled || updating"
            @click="openPlugin(p)"
          >
            打开
          </button>
          <button class="pc__btn" :disabled="!!pending[p.manifest.name] || updating" @click="toggle(p)">
            {{ p.enabled ? "停用" : "启用" }}
          </button>
          <button class="pc__btn" :disabled="!!pending[p.manifest.name] || updating" @click="reveal(p)">
            打开目录
          </button>
          <button
            class="pc__btn pc__btn--danger"
            :disabled="!!pending[p.manifest.name] || updating"
            @click="remove(p)"
          >
            卸载
          </button>
        </div>
      </li>
    </ul>
    <p v-if="actionError || plugins.error" class="pc__error">{{ actionError || plugins.error }}</p>
  </section>
</template>

<style scoped src="./plugin-center.css"></style>

<style scoped>
.pc__empty {
  padding: 24px;
  text-align: center;
  font-size: 12px;
  color: var(--text-dim-strong);
  border: 1px dashed var(--panel-control-border);
  border-radius: 8px;
}

.pc__ilist {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
}

.pc__iitem:hover {border-color:var(--accent);}

.pc__iitem:focus-visible {outline:2px solid var(--accent);}

.pc__iitem {
  cursor:pointer;
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px 14px;
  border: 1px solid var(--panel-control-border);
  border-radius: 10px;
}

.pc__iitem-icon {
  width: 40px;
  height: 40px;
  flex: none;
  border-radius: 8px;
  overflow: hidden;
  background: var(--hover-bg);
}

.pc__iitem-icon-img {
  width: 40px;
  height: 40px;
  object-fit: contain;
}

.pc__iitem-body {
  flex: 1;
  min-width: 0;
}

.pc__iitem-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.pc__iitem-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--panel-fg);
}

.pc__badge {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 10px;
  border: 1px solid var(--accent);
  color: var(--accent);
}

.pc__badge--off {
  border-color: var(--text-dim-strong);
  color: var(--text-dim-strong);
}

.pc__badge--origin {
  border-color: var(--text-dim-strong);
  color: var(--text-dim-strong);
}

.pc__badge--official {
  border-color: var(--accent);
  color: var(--accent);
}

.pc__badge--warn {
  border-color: #d9534f;
  color: #d9534f;
}

.pc__iitem-warn {
  margin: 2px 0 0;
  font-size: 11px;
  color: #d9534f;
}

.pc__star {
  border: none;
  background: transparent;
  color: var(--text-dim-strong);
  font-size: 13px;
  cursor: pointer;
  line-height: 1;
  padding: 2px 4px;
  border-radius: 4px;
}

.pc__star:hover {
  color: #e5c07b;
}

.pc__star--on {
  color: #e5c07b;
}

.pc__iitem-desc {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pc__iitem-meta {
  margin: 2px 0 0;
  font-size: 11px;
  color: var(--text-dim-strong);
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
}

.pc__iactions {
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex: none;
}
</style>
