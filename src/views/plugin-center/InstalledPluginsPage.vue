<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useToolCatalogStore } from "@/stores/toolCatalog";
import TopTabNav from "@/components/TopTabNav.vue";
import { pluginDisplayName } from "@/lib/pluginPresentation";
import { fallbackIconDataUri } from "@/lib/iconFallback";
import { safeInvoke } from "@/lib/tauri";
import { usePluginsStore, type LoadedPlugin, type PluginOrigin } from "@/stores/plugins";
import { useUsageStore } from "@/stores/usage";
import { useCenterSearchStore } from "@/stores/centerSearch";

const props = defineProps<{ detached: boolean }>();
const emit = defineEmits<{ detail: [name: string] }>();
const plugins = usePluginsStore();
const catalog = useToolCatalogStore();
onMounted(()=>{void plugins.reload();void catalog.refresh();});
function updateEntry(name:string) { return catalog.updates.find(entry=>entry.name===name); }
async function checkUpdates(){await Promise.all([plugins.reload(),catalog.refresh(true)]);}
const usage = useUsageStore();
const search = useCenterSearchStore();
const router = useRouter();
const enableFilter = ref("all");
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
    <div class="pc__row"><button class="pc__btn" :disabled="plugins.loading || catalog.loading" @click="checkUpdates">{{ catalog.loading?'检查中…':'检查更新 / 重新扫描' }}</button><button v-if="catalog.updates.length" class="pc__btn pc__btn--primary" :disabled="!!catalog.busy || catalog.updatingAll" @click="catalog.updateAll()">{{ catalog.updatingAll?'更新中…':`全部更新（${catalog.updates.length}）` }}</button></div>
    <p v-if="catalog.loadError" class="pc__error">{{ catalog.loadError }}</p><p v-if="catalog.notice" role="status">{{ catalog.notice }}</p><p v-if="catalog.error" class="pc__error">{{ catalog.error }}</p>
    <TopTabNav v-model="enableFilter" :tabs="installTabs" />
    <div v-if="plugins.plugins.length === 0 && !plugins.loading" class="pc__empty">
      暂无插件。切换到「开发者」安装工具包，或从本地目录安装。
    </div>
    <ul v-else class="pc__ilist">
      <li v-for="p in installedPlugins" :key="p.manifest.name" class="pc__iitem" role="button" tabindex="0" @click="emit('detail', p.manifest.name)" @keydown.enter.self="emit('detail', p.manifest.name)" @keydown.space.self.prevent="emit('detail', p.manifest.name)">
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
          <button v-if="updateEntry(p.manifest.name)" class="pc__btn pc__btn--primary" :disabled="!!catalog.busy || !!pending[p.manifest.name]" @click="catalog.install(updateEntry(p.manifest.name)!)">{{ catalog.busy===p.manifest.name?'更新中…':'更新' }}</button>
          <button
            class="pc__btn pc__btn--primary"
            :disabled="!p.enabled"
            @click="openPlugin(p)"
          >
            打开
          </button>
          <button class="pc__btn" :disabled="!!pending[p.manifest.name] || !!catalog.busy" @click="toggle(p)">
            {{ p.enabled ? "停用" : "启用" }}
          </button>
          <button class="pc__btn" :disabled="!!pending[p.manifest.name] || !!catalog.busy" @click="reveal(p)">
            打开目录
          </button>
          <button
            class="pc__btn pc__btn--danger"
            :disabled="!!pending[p.manifest.name] || !!catalog.busy"
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
