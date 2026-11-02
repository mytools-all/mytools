<template>
  <section class="my-panel" @keydown.esc="close">
    <header data-tauri-drag-region="deep"><strong>我的面板</strong><span>鼠标中键 · 快速启动</span><button aria-label="关闭面板" @click="close">×</button></header>
    <div class="panel-tools"><input v-model="query" placeholder="搜索面板入口…" aria-label="搜索面板入口" /><button @click="editing = !editing">{{ editing ? '完成' : '编辑' }}</button><button @click="adding = !adding">＋ 添加</button></div>
    <div v-if="adding" class="add-form">
      <select v-model="kind" aria-label="入口类型"><option value="plugin">插件功能</option><option value="app">应用</option><option value="path">文件 / 文件夹</option><option value="url">网址</option></select>
      <select v-if="kind === 'plugin'" v-model="target" aria-label="选择插件功能"><option value="">选择插件功能</option><option v-for="option in pluginOptions" :key="option.target" :value="option.target">{{ option.title }}</option></select>
      <select v-else-if="kind === 'app'" v-model="target" aria-label="选择应用"><option value="">选择应用</option><option v-for="app in apps.entries" :key="app.target" :value="app.target">{{ app.name }}</option></select>
      <input v-else v-model="target" :placeholder="kind === 'url' ? 'https://…' : '文件或文件夹绝对路径'" aria-label="入口目标" />
      <div v-if="kind === 'path'"><button @click="browse(false)">选择文件</button><button @click="browse(true)">选择文件夹</button></div>
      <input v-if="kind !== 'plugin'" v-model="title" placeholder="显示名称（可选）" aria-label="入口名称" /><button @click="add">添加到面板</button>
    </div>
    <div class="panel-grid" role="list">
      <article v-for="entry in visible" :key="entry.id" :draggable="editing" role="listitem" @dragstart="dragged = entry.id" @dragover.prevent @drop.prevent="reorder(entry.id)">
        <button class="entry" :title="entry.target" @click="launch(entry)"><img v-if="icon(entry)" :src="icon(entry)" alt="" /><span v-else class="entry-icon">{{ entry.kind === 'url' ? '↗' : entry.kind === 'path' ? '▱' : label(entry).slice(0, 1) }}</span><span>{{ label(entry) }}</span></button>
        <div v-if="editing" class="entry-edit"><button aria-label="向前移动" @click="moveEarlier(entry.id)">←</button><button aria-label="移除入口" @click="remove(entry.id)">移除</button></div>
      </article>
      <p v-if="!visible.length" class="empty">{{ entries.length ? '没有匹配的入口' : '点击“添加”，把常用插件、应用和文件放到这里' }}</p>
    </div>
    <footer><label><input :checked="middleEnabled" type="checkbox" @change="toggleMiddle" />中键唤出</label><span>{{ message || (editing ? '拖动入口调整顺序' : 'Esc 或点击面板外关闭') }}</span></footer>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useEventListener } from "@vueuse/core";
import { storeToRefs } from "pinia";
import { usePanelStore } from "@/stores/panel";
import { fallbackIconDataUri } from "@/lib/iconFallback";
import { usePluginsStore } from "@/stores/plugins";
import { useAppIndexStore } from "@/stores/appIndex";
import { safeInvoke, tauriAvailable } from "@/lib/tauri";
import { movePanelEntry, validPanelEntry, type PanelEntry } from "@/lib/panelEntries";
import { pluginDisplayName } from "@/lib/pluginPresentation";

const plugins = usePluginsStore();
const apps = useAppIndexStore();
const { entries } = storeToRefs(usePanelStore());
const query = ref("");
const editing = ref(false);
const adding = ref(false);
const kind = ref<PanelEntry["kind"]>("plugin");
const target = ref("");
const title = ref("");
const dragged = ref("");
const message = ref("");
const middleEnabled = ref(true);
const pluginOptions = computed(() => plugins.enabledPlugins.flatMap(p => p.manifest.features.map(f => ({target: p.manifest.name + ":" + f.code, title: pluginDisplayName(p.manifest) + (p.manifest.features.length > 1 ? " · " + (f.explain || f.code) : "")}))));
const visible = computed(() => (Array.isArray(entries.value) ? entries.value : []).filter(validPanelEntry).filter(e => (label(e) + e.target).toLowerCase().includes(query.value.toLowerCase())));
watch(kind, () => { target.value = ""; });
function label(entry: PanelEntry) {
  const plugin = entry.kind === "plugin" ? plugins.plugins.find(p => p.manifest.name === entry.target) : undefined;
  if (!plugin) return entry.title;
  const feature = plugin.manifest.features.find(f => f.code === entry.feature);
  return pluginDisplayName(plugin.manifest) + (plugin.manifest.features.length > 1 && feature ? " · " + (feature.explain || feature.code) : "");
}
function icon(entry: PanelEntry) { return entry.kind === "plugin" ? plugins.plugins.find(p => p.manifest.name === entry.target)?.icon || fallbackIconDataUri(label(entry)) : entry.kind === "app" ? apps.iconFor(entry.target) || "" : ""; }
function add() {
  const option = pluginOptions.value.find(x => x.target === target.value);
  const app = apps.entries.find(x => x.target === target.value);
  const [plugin, feature] = target.value.split(":");
  const entry: PanelEntry = {id: crypto.randomUUID(), kind: kind.value, target: kind.value === "plugin" ? plugin || "" : target.value.trim(), title: title.value.trim() || option?.title || app?.name || target.value.split(/[\\/]/).filter(Boolean).pop() || "入口", feature: kind.value === "plugin" ? feature : undefined};
  if (!validPanelEntry(entry)) { message.value = "请填写有效入口；网址仅支持 http / https，文件需绝对路径"; return; }
  if (entries.value.some(e => e.kind === entry.kind && e.target === entry.target && e.feature === entry.feature)) { message.value = "该入口已在面板中"; return; }
  entries.value = [...entries.value, entry];
  target.value = ""; title.value = ""; adding.value = false; message.value = "已添加";
}
function remove(id: string) { entries.value = entries.value.filter(e => e.id !== id); }
function reorder(id: string) { entries.value = movePanelEntry(entries.value, dragged.value, id); dragged.value = ""; }
function moveEarlier(id: string) { const at = entries.value.findIndex(e => e.id === id); const previous = entries.value[at - 1]; if (previous) entries.value = movePanelEntry(entries.value, id, previous.id); }
async function close() { if (tauriAvailable()) { const {getCurrentWindow} = await import("@tauri-apps/api/window"); await getCurrentWindow().hide(); } }
useEventListener(document, "keydown", event => { if (event.key === "Escape") void close(); });
async function browse(directory: boolean) {
  try {
    const {open} = await import("@tauri-apps/plugin-dialog");
    const picked = await open({directory, multiple:false});
    if (typeof picked === "string") target.value = picked;
  } catch (error) { message.value = String(error); }
}
async function launch(entry: PanelEntry) {
  if (editing.value) return;
  try {
    if (entry.kind === "plugin") await safeInvoke("plugin_open_window", {pluginId: entry.target, feature: entry.feature});
    else await safeInvoke("app_launcher_open", {path: entry.target});
    await close();
  } catch (error) { message.value = error instanceof Error ? error.message : String(error); }
}
async function toggleMiddle(event: Event) {
  const enabled = (event.target as HTMLInputElement).checked;
  try { await safeInvoke("panel_set_middle_enabled", {options: {enabled}}); middleEnabled.value = enabled; message.value = enabled ? "中键唤出已开启" : "已恢复鼠标中键原有行为"; }
  catch (error) { message.value = String(error); }
}
onMounted(async () => {
  await plugins.ensureLoaded();
  void apps.ensureLoaded();
  if (!Array.isArray(entries.value)) entries.value = [];
  entries.value = entries.value.filter(validPanelEntry);
  if (!localStorage.getItem("mytools-panel-initialized-v1") && !entries.value.length) entries.value = pluginOptions.value.slice(0, 8).map(option => {
    const [target, feature] = option.target.split(":");
    return {id:crypto.randomUUID(), title:option.title, kind:"plugin" as const, target:target || "", feature};
  });
  localStorage.setItem("mytools-panel-initialized-v1", "true");
  middleEnabled.value = await safeInvoke<boolean>("panel_middle_enabled") ?? true;
});
</script>

<style scoped>
.my-panel { width:100%; min-width:0; flex:1; height:100%; box-sizing:border-box; font-family:"Segoe UI","Microsoft YaHei",sans-serif; display:flex; flex-direction:column; color:var(--panel-fg); background:var(--panel-bg-strong); border:1px solid var(--panel-border); border-radius:12px; overflow:hidden; }
header { display:flex; align-items:center; gap:12px; padding:14px; border-bottom:1px solid var(--panel-border); }
header span,footer { color:var(--text-dim); font-size:11px; }
header button { margin-left:auto; }
button,input,select { color:inherit; background:var(--panel-control-bg); border:1px solid var(--panel-control-border); border-radius:7px; padding:7px; }
button { cursor:pointer; } button:hover { background:var(--hover-bg); }
.panel-tools { display:flex; gap:6px; padding:10px; }.panel-tools input { flex:1; min-width:0; }
.add-form { display:grid; grid-template-columns:1fr 1fr; gap:6px; padding:0 10px 10px; }.add-form input,.add-form select { min-width:0; }
.panel-grid { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); align-content:start; gap:6px; padding:8px 12px; flex:1; overflow:auto; }
.entry { border:0; background:transparent; display:flex; width:100%; flex-direction:column; align-items:center; gap:8px; font-size:12px; overflow-wrap:anywhere; min-height:85px; }
.entry img,.entry-icon { width:34px; height:34px; object-fit:contain; }.entry-icon { display:grid; place-items:center; font-size:23px; color:var(--accent); background:var(--accent-soft); border-radius:10px; }
.entry-edit { display:flex; gap:3px; justify-content:center; }.entry-edit button { font-size:10px; padding:3px; }
footer { padding:10px 12px; display:flex; align-items:center; justify-content:space-between; gap:10px; border-top:1px solid var(--panel-border); }footer label { white-space:nowrap; }footer input { vertical-align:middle; }
.empty { grid-column:1/-1; padding:30px 10px; text-align:center; font-size:13px; color:var(--text-dim); }
</style>
