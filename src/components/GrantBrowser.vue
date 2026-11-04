<script setup lang="ts">
import { computed, ref } from "vue";
import { usePluginsStore } from "@/stores/plugins";
import { pluginDisplayName } from "@/lib/pluginPresentation";

export interface GrantRow { plugin: string; permission: string }
const props = defineProps<{
  title: string;
  description: string;
  directory?: boolean;
  records: GrantRow[];
  loading: boolean;
  error: string | null;
  revoke: (plugin: string, permission: string) => Promise<void>;
}>();
const emit = defineEmits<{refresh: []}>();
const plugins = usePluginsStore();
const query = ref("");
const grouping = ref("plugin");
const pending = ref("");
const actionError = ref("");
const labels: Record<string,string> = {
  "clipboard.read":"读取剪贴板", "clipboard.write":"写入剪贴板", "clipboard.history":"剪贴板历史", "clipboard.remove":"删除剪贴板历史",
  "fs.read":"读取文件", "fs.write":"写入文件", "fs.search":"目录搜索", "fs.globalsearch":"全局文件搜索", "fs.searchpreview":"文件预览",
  "fs.pickdirectory":"选择目录", "fs.pickfiles":"选择文件", "fs.exporttext":"导出文件", "fs.importdropped":"导入拖放文件", "fs.renamemany":"批量重命名", "fs.undorename":"撤销重命名",
  "net.fetch":"网络请求", "context.read":"读取面板上下文", "shell.open":"打开文件或网址", "screen.pickcolor":"屏幕取色",
};
function name(id: string) { const plugin = plugins.plugins.find(item => item.manifest.name === id); return plugin ? pluginDisplayName(plugin.manifest) : id; }
function permissionName(permission: string) { return props.directory ? "目录访问" : labels[permission] || permission; }
const filtered = computed(() => {
  const words = query.value.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  return props.records.filter(record => {
    const text = [name(record.plugin),record.plugin,record.permission,permissionName(record.permission)].join(" ").toLocaleLowerCase();
    return words.every(word => text.includes(word));
  });
});
const groups = computed(() => {
  const result = new Map<string,GrantRow[]>();
  for (const record of filtered.value) {
    const key = grouping.value === "plugin" ? record.plugin : record.permission;
    if (!result.has(key)) result.set(key,[]);
    result.get(key)!.push(record);
  }
  return [...result].map(([id,records]) => ({id,records,title:grouping.value === "plugin" ? name(id) : props.directory ? id : permissionName(id)}))
    .sort((a,b) => a.title.localeCompare(b.title,"zh-CN"));
});
async function revokeRecord(record: GrantRow) {
  pending.value = JSON.stringify([record.plugin,record.permission]);
  actionError.value = "";
  try { await props.revoke(record.plugin,record.permission); }
  catch(error) { actionError.value = String(error); }
  finally { pending.value = ""; }
}
</script>

<template>
  <section class="grants" :aria-label="title">
    <header class="grants__header"><div><h4>{{ title }}</h4><span class="grants__count">已授权 {{ records.length }} {{ directory ? '个目录' : '项权限' }}</span></div><button :disabled="loading || !!pending" @click="emit('refresh')">{{ loading ? '刷新中…' : '刷新' }}</button></header>
    <p class="grants__description">{{ description }}</p>
    <div class="grants__tools">
      <div class="grants__search"><input v-model="query" :aria-label="directory ? '搜索目录授权' : '搜索插件授权'" :placeholder="directory ? '搜索插件名称、标识或授权目录…' : '搜索插件名称、标识或权限…'" /><button v-if="query" aria-label="清空授权搜索" @click="query=''">×</button></div>
      <select v-model="grouping" :aria-label="directory ? '目录授权分组' : '插件授权分组'"><option value="plugin">按插件分组</option><option value="permission">{{ directory ? '按授权目录分组' : '按权限分组' }}</option></select>
    </div>
    <p v-if="query.trim()" class="grants__count" role="status">匹配 {{ filtered.length }} / {{ records.length }} 项</p>
    <section v-for="group in groups" :key="group.id" class="grants__group">
      <h5 :title="group.title"><span>{{ group.title }}</span><small>{{ group.records.length }}</small></h5>
      <ul>
        <li v-for="record in group.records" :key="JSON.stringify([record.plugin,record.permission])">
        <div class="grants__record"><strong>{{ grouping === 'plugin' ? permissionName(record.permission) : name(record.plugin) }}</strong><code v-if="grouping === 'plugin' || directory" :title="record.permission">{{ record.permission }}</code><code v-else>{{ record.plugin }}</code></div>
        <button class="grants__revoke" :disabled="loading || !!pending" :aria-label="`撤销 ${name(record.plugin)} 的 ${record.permission}`" @click="revokeRecord(record)">{{ pending === JSON.stringify([record.plugin,record.permission]) ? '撤销中…' : '撤销' }}</button>
        </li>
      </ul>
    </section>
    <p v-if="!filtered.length && !loading" class="grants__empty">{{ records.length ? '没有匹配的授权，请调整搜索条件' : '暂无授权记录' }}</p>
    <p v-if="error || actionError" class="grants__error" role="alert">{{ error || actionError }}</p>
  </section>
</template>

<style scoped>
.grants { display:flex; flex-direction:column; flex-shrink:0; gap:12px; padding:18px; border:1px solid var(--panel-control-border); border-radius:10px; background:var(--surface-bg); min-width:0; }
.grants__header { display:flex; align-items:center; justify-content:space-between; gap:12px; }
.grants__header>div { display:flex; align-items:center; flex-wrap:wrap; gap:12px; }
h4,h5,p { margin:0; }h4 { font-size:13px; font-weight:600; }
.grants__count,.grants__description,.grants__empty { font-size:12px; color:var(--text-dim); line-height:1.6; }
.grants button,.grants select { flex:none; border:1px solid var(--panel-control-border); border-radius:7px; background:var(--surface-bg); color:var(--panel-fg); padding:7px 12px; font-size:12px; cursor:pointer; }
.grants button:disabled { opacity:.5; cursor:default; }.grants button:hover:not(:disabled) { background:var(--hover-bg); }
.grants__tools { display:flex; gap:8px; align-items:center; flex-wrap:wrap; }
.grants__search { flex:1 1 200px; min-width:0; display:flex; align-items:center; height:36px; border:1px solid var(--panel-control-border); border-radius:7px; }
.grants__search:focus-within { border-color:var(--accent); }.grants__search input { flex:1; min-width:0; width:100%; height:100%; padding:0 10px; border:0; outline:none; color:var(--panel-fg); background:transparent; font-size:12px; }
.grants__search button { border:0; padding:3px 9px; background:transparent; }
.grants__tools select { height:36px; }.grants__group { min-width:0; border:1px solid var(--panel-border); border-radius:8px; overflow:hidden; }
h5 { display:flex; align-items:center; gap:8px; padding:9px 12px; background:var(--chip-bg); font-size:12px; }h5 span { min-width:0; overflow-wrap:anywhere; }h5 small { flex:none; color:var(--text-dim); font-weight:400; }
ul { list-style:none; margin:0; padding:0; }li { display:flex; align-items:center; gap:12px; padding:10px 12px; }li+li { border-top:1px solid var(--panel-border); }
.grants__record { flex:1; min-width:0; display:flex; flex-direction:column; gap:3px; }.grants__record strong { font-size:12px; font-weight:500; }.grants__record code { font-size:11px; color:var(--text-dim); overflow-wrap:anywhere; }
.grants .grants__revoke,.grants__error { color:var(--danger); }.grants__error { font-size:12px; }.grants__empty { padding:12px 0; text-align:center; }
</style>
