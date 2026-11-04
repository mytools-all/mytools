<script setup lang="ts">
import { computed, ref, watch, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToolCatalogStore } from "@/stores/toolCatalog";
import { usePluginsStore } from "@/stores/plugins";
import { useUsageStore } from "@/stores/usage";
import { pluginDisplayName } from "@/lib/pluginPresentation";
import { fallbackIconDataUri } from "@/lib/iconFallback";
import { safeInvoke, isDetachedCenter } from "@/lib/tauri";

const props = defineProps<{ name: string; embedded?: boolean }>();
const route=useRoute();
const router=useRouter();
const catalog=useToolCatalogStore();
const plugins=usePluginsStore();
const usage=useUsageStore();
const error=ref("");
const pending=ref(false);
const confirmUninstall=ref(false);
const notice=ref("");
watch(()=>props.name,()=>{confirmUninstall.value=false;error.value="";notice.value="";});
onMounted(()=>{void catalog.refresh();});
const entry=computed(()=>catalog.entries.find(e=>e.name===props.name));
const installed=computed(()=>plugins.plugins.find(p=>p.manifest.name===props.name));
const title=computed(()=>entry.value?.title || (installed.value ? pluginDisplayName(installed.value.manifest) : props.name));
const features=computed(()=>entry.value?.features || installed.value?.manifest.features.map(f=>f.explain || f.code) || []);
const capabilities=computed(()=>installed.value?.declaredCapabilities || entry.value?.capabilities || []);
const status=computed(()=>installed.value ? installed.value.incompatible ? "不兼容" : installed.value.enabled ? "已安装" : "已停用" : entry.value?.status==='builtin' ? "随应用内置" : entry.value?.status==='ready' ? "可安装" : "开发中");
const notes=computed(()=>entry.value?.notes.length ? entry.value.notes : ["安装后点击“打开”进入插件，也可从主页搜索启动。","涉及真实文件、剪贴板或网络的功能按能力声明申请授权。"]);
function back(){const query={...route.query};delete query.detail;void router.push({path:"/plugins",query});}
async function run(action:()=>Promise<unknown>){pending.value=true;error.value="";try{await action();}catch(e){error.value=String(e);}finally{pending.value=false;}}
async function uninstallPlugin(){await run(async()=>{if(!await plugins.uninstall(props.name))throw Error(plugins.error || "卸载失败");confirmUninstall.value=false;notice.value="已卸载插件，插件存储的数据仍保留。";});}
function openPlugin(){const p=installed.value;const feature=p?.manifest.features[0];if(!p||!feature||!p.enabled||p.incompatible)return;usage.bump(p.manifest.name);if(isDetachedCenter()){void run(()=>safeInvoke("plugin_open_window",{pluginId:p.manifest.name,feature:feature.code}));return;}void router.push({name:"plugin",params:{id:p.manifest.name,code:feature.code}});}
function safeUrl(url?:string){try{const parsed=new URL(url || "");return ["https:","http:"].includes(parsed.protocol)?parsed.href:undefined;}catch{return undefined;}}
</script>

<template>
  <article class="pd">
    <button v-if="!embedded" class="pd__back" @click="back">← 返回{{ route.query.section === 'installed' ? '已安装' : '插件商店' }}</button>
    <template v-if="entry || installed">
      <header class="pd__hero">
        <img :src="installed?.icon || entry?.icon || fallbackIconDataUri(title)" :alt="title" />
        <div class="pd__heading"><h1>{{ title }}</h1><p>{{ entry?.description || installed?.manifest.description }}</p><span>{{ status }} · v{{ installed?.manifest.version || entry?.version }}<template v-if="entry"> · {{ entry.category }}</template></span></div>
        <div class="pd__actions">
          <button v-if="installed" class="pd__primary" :disabled="!installed.enabled || !!installed.incompatible || !installed.manifest.features.length" @click="openPlugin">打开</button>
          <button v-else-if="entry?.status === 'ready'" class="pd__primary" :disabled="catalog.busy === name" @click="catalog.install(entry)">{{ catalog.busy === name ? '安装中…' : '安装' }}</button>
          <button v-if="installed" :disabled="pending" @click="run(()=>plugins.setEnabled(name,!installed?.enabled))">{{ installed.enabled ? '停用' : '启用' }}</button>
          <button v-if="installed" :disabled="pending" @click="run(()=>plugins.openDir(name))">打开目录</button>
          <button v-if="installed" class="pd__uninstall" :disabled="pending" @click="confirmUninstall=true">卸载</button>
        </div>
      </header>
      <div v-if="confirmUninstall && installed" class="pd__confirm" role="group" aria-label="确认卸载插件"><p>确认卸载 {{ title }}？只移除插件程序，保留插件存储的数据。</p><button :disabled="pending" @click="confirmUninstall=false">取消</button><button class="pd__uninstall" :disabled="pending" @click="uninstallPlugin">{{ pending ? '卸载中…' : '确认卸载' }}</button></div>
      <p v-if="notice" role="status">{{ notice }}</p>
      <p v-if="error || catalog.error" class="pd__error" role="alert">{{ error || catalog.error }}</p>
      <p v-if="installed?.incompatible" class="pd__error">{{ installed.incompatible }}</p>
      <div class="pd__layout">
<div class="pd__content">
        <section><h2>插件介绍</h2><p>{{ entry?.description || installed?.manifest.description || '作者暂未提供详细说明。' }}</p><ul><li v-for="feature in features" :key="feature">{{ feature }}</li></ul></section>
        <section><h2>使用说明</h2><ol><li v-for="note in notes" :key="note">{{ note }}</li></ol></section>
        <section v-if="installed?.manifest.features.length"><h2>启动关键词</h2><div v-for="feature in installed.manifest.features" :key="feature.code" class="pd__keywords"><strong>{{ feature.explain || feature.code }}</strong><span v-for="word in feature.cmds" :key="word">{{ word }}</span></div></section>
        <section><h2>权限与数据</h2><p v-if="!capabilities.length">未提供能力声明，请留意运行时的授权提示。</p><p v-else>以下能力由宿主统一管理；需要确认的操作会在使用时请求授权。</p><div class="pd__caps"><code v-for="cap in capabilities" :key="cap">{{ cap }}</code></div></section>
      </div><aside class="pd__info">
        <section><h2>插件信息</h2><dl><dt>包标识</dt><dd>{{ name }}</dd><dt>版本</dt><dd>{{ installed?.manifest.version || entry?.version }}</dd><dt v-if="entry?.author || installed?.manifest.author">作者</dt><dd v-if="entry?.author || installed?.manifest.author">{{ entry?.author || installed?.manifest.author }}</dd><dt v-if="entry">npm 包</dt><dd v-if="entry">{{ entry.npm }}</dd><dt v-if="entry">许可证</dt><dd v-if="entry">{{ entry.license }}</dd><dt>来源</dt><dd>{{ installed?.origin === 'official' ? '随应用提供' : entry ? '独立工具包' : '已安装插件' }}</dd></dl></section>
        <section v-if="entry"><h2>安装与分发</h2><p v-if="entry.status === 'builtin'">随应用提供，无需单独安装。</p><template v-else><p>通过 npm 安装独立插件包，自动安装到统一的插件根目录，无需为每个插件选择位置。</p><p>拿到插件目录 / zip / tgz 后，可在“开发者”中安装。复制目录本身不会自动升级现有安装。</p></template></section>
        <section v-if="entry || safeUrl(installed?.manifest.homePage)"><h2>相关链接</h2><a v-if="entry" :href="entry.repository" target="_blank" rel="noopener noreferrer">插件仓库 ↗</a><a v-if="entry" :href="'https://www.npmjs.com/package/'+entry.npm" target="_blank" rel="noopener noreferrer">npm 包 ↗</a><a v-if="safeUrl(installed?.manifest.homePage)" :href="safeUrl(installed?.manifest.homePage)" target="_blank" rel="noopener noreferrer">作者主页 ↗</a><a v-if="entry?.reference" :href="safeUrl(entry.reference.url)" target="_blank" rel="noopener noreferrer">交互参考：{{ entry.reference.label }} ↗</a><p v-if="entry?.reference" class="pd__muted">参考来源仅用于交互设计，不是本插件的运行依赖。</p></section>
      </aside>
</div>
    </template>
    <p v-else class="pd__missing">{{ notice || '未找到此插件，可能已卸载或不在当前工具目录中。' }}</p>
  </article>
</template>

<style scoped>
.pd{overflow-y:auto;min-height:0;padding:4px 4px 24px;color:var(--panel-fg);}.pd button{border:1px solid var(--panel-control-border);border-radius:7px;background:var(--surface-bg);color:inherit;padding:7px 12px;cursor:pointer;}.pd button:disabled{opacity:.5;cursor:default;}.pd__back{margin-bottom:20px;}.pd__hero{display:flex;gap:18px;align-items:flex-start;padding:8px 0 24px;border-bottom:1px solid var(--panel-border);}.pd__hero>img{width:64px;height:64px;object-fit:contain;border-radius:14px;}.pd__heading{flex:1;min-width:0;}.pd h1{font-size:23px;margin:0 0 10px;}.pd p,.pd li{font-size:13px;line-height:1.8;white-space:pre-wrap;overflow-wrap:anywhere;}.pd__heading p{margin:0 0 7px;color:var(--text-dim);}.pd__heading span,.pd__muted{font-size:11px!important;color:var(--text-dim-strong);}.pd__actions{display:flex;flex-wrap:wrap;gap:7px;max-width:180px;}.pd .pd__primary{background:var(--accent);color:#fff;border-color:var(--accent);}.pd__layout{display:grid;grid-template-columns:minmax(0,1fr) 240px;gap:28px;}.pd section{padding:20px 0;border-bottom:1px solid var(--panel-border);}.pd h2{font-size:15px;margin:0 0 12px;}.pd ul,.pd ol{padding-left:20px;}.pd__caps,.pd__keywords{display:flex;gap:6px;flex-wrap:wrap;margin-bottom:8px;}.pd code,.pd__keywords span{font-size:11px;padding:4px 7px;border-radius:5px;background:var(--chip-bg);overflow-wrap:anywhere;}.pd__keywords strong{font-size:12px;width:100%;}.pd dl{font-size:12px;}.pd dt{color:var(--text-dim);margin-top:12px;}.pd dd{margin:5px 0 0;overflow-wrap:anywhere;}.pd a{display:block;font-size:12px;color:var(--accent);line-height:1.8;overflow-wrap:anywhere;margin-bottom:8px;}.pd__error{color:var(--danger,#df6565);}.pd .pd__uninstall{color:var(--danger,#df6565);}.pd__confirm{padding:12px;border:1px solid var(--panel-border);border-radius:8px;}.pd__confirm button{margin-right:8px;}.pd__missing{padding:30px;text-align:center;}@media(max-width:750px){.pd__layout{grid-template-columns:1fr;gap:0;}.pd__hero{flex-wrap:wrap;}.pd__actions{max-width:none;}}
</style>
