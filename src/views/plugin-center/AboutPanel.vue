<script setup lang="ts">
import { onMounted, ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useCatalogSourcesStore } from "@/stores/catalogSources";
import { APP_INFO } from "@/config/app";
import { tauriAvailable } from "@/lib/tauri";
import appIcon from "../../../src-tauri/icons/128x128.png";

const sources = useCatalogSourcesStore();
onMounted(() => { void sources.load().catch(reason => { error.value = String(reason); }); });
const error = ref("");
async function openLink(event: MouseEvent, url: string) {
  if (!tauriAvailable()) return;
  event.preventDefault();
  error.value = "";
  try { await openUrl(url); }
  catch (reason) { error.value = `无法打开链接：${reason instanceof Error ? reason.message : String(reason)}`; }
}
</script>

<template>
  <section class="about" :aria-label="`关于 ${APP_INFO.name}`">
    <header class="about__header">
      <img :src="appIcon" :alt="APP_INFO.name" width="56" height="56" />
      <div>
        <h2>{{ APP_INFO.name }} <span class="about__version">v{{ APP_INFO.version }}</span></h2>
        <p>{{ APP_INFO.description }}</p>
      </div>
    </header>
    <dl class="about__info">
      <dt>作者</dt>
      <dd><a :href="APP_INFO.authorUrl" target="_blank" rel="noopener noreferrer" @click="openLink($event, APP_INFO.authorUrl)">{{ APP_INFO.author }} ↗</a></dd>
      <dt>当前版本</dt><dd>{{ APP_INFO.version }}</dd>
      <dt>支持平台</dt><dd>{{ APP_INFO.platform }}</dd>
      <dt>开源许可</dt><dd>{{ APP_INFO.license }}</dd>
      <dt>项目仓库</dt>
      <dd><a :href="APP_INFO.repository" target="_blank" rel="noopener noreferrer" @click="openLink($event, APP_INFO.repository)">{{ APP_INFO.repository }} ↗</a></dd>
      <dt>插件列表</dt><dd><a :href="sources.pluginList" target="_blank" rel="noopener noreferrer" @click="openLink($event, sources.pluginList)">{{ sources.pluginList }} ↗</a></dd>
      <dt>插件列表镜像</dt><dd><a :href="sources.pluginListMirror" target="_blank" rel="noopener noreferrer" @click="openLink($event, sources.pluginListMirror)">{{ sources.pluginListMirror }} ↗</a></dd>
    </dl>
    <nav class="about__links" aria-label="项目链接">
      <a :href="APP_INFO.homepage" target="_blank" rel="noopener noreferrer" @click="openLink($event, APP_INFO.homepage)">项目文档 ↗</a>
      <a :href="APP_INFO.issues" target="_blank" rel="noopener noreferrer" @click="openLink($event, APP_INFO.issues)">反馈问题 ↗</a>
    </nav>

    <p v-if="error" class="about__error" role="alert">{{ error }}</p>
  </section>
</template>

<style scoped>
.about { display:flex; flex-shrink:0; flex-direction:column; gap:22px; padding:24px; background:var(--surface-bg); border:1px solid var(--panel-control-border); border-radius:12px; }
.about__header { display:flex; align-items:center; gap:16px; }
.about__header img { flex:none; }
.about__header h2 { display:flex; align-items:center; flex-wrap:wrap; gap:10px; margin:0; font-size:21px; color:var(--panel-fg); }
.about__header p { margin:7px 0 0; font-size:13px; color:var(--text-dim); }
.about__version { padding:3px 8px; border-radius:6px; background:var(--accent-soft); color:var(--accent); font-size:12px; font-weight:500; }
.about__info { display:grid; grid-template-columns:max-content minmax(0,1fr); gap:14px 24px; margin:0; font-size:13px; line-height:1.6; }
.about__info dt { color:var(--text-dim-strong); }
.about__info dd { margin:0; min-width:0; color:var(--panel-fg); overflow-wrap:anywhere; }
.about a { color:var(--accent); text-decoration:none; }
.about a:hover { text-decoration:underline; }
.about a:focus-visible { outline:2px solid var(--accent); outline-offset:3px; border-radius:3px; }
.about__links { display:flex; flex-wrap:wrap; gap:18px; padding-top:18px; border-top:1px solid var(--panel-control-border); font-size:13px; }
.about__error { margin:0; font-size:12px; color:#e06c75; }
@media(max-width:600px) { .about { padding:18px; gap:18px; } .about__info { gap:12px 16px; } }
</style>
