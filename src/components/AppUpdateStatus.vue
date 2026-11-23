<script setup lang="ts">
import { onMounted, ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useAppUpdatesStore } from "@/stores/appUpdates";

defineProps<{ compact?: boolean }>();
const updates = useAppUpdatesStore();
const confirming = ref(false);
const linkError = ref("");
const vFocus = { mounted: (element: HTMLElement) => element.focus() };
function trapFocus(event: KeyboardEvent) {
  const buttons = (event.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>("button");
  if ((!event.shiftKey && event.target === buttons[buttons.length - 1]) || (event.shiftKey && event.target === buttons[0])) {
    event.preventDefault();
    buttons[event.shiftKey ? buttons.length - 1 : 0]?.focus();
  }
}
onMounted(() => { void updates.initialize(); });

async function openRelease() {
  if (!updates.state.releaseUrl) return;
  try { await openUrl(updates.state.releaseUrl); }
  catch (reason) { linkError.value = `打开发布页失败：${reason}`; }
}

async function install() {
  confirming.value = false;
  await updates.install();
}
</script>

<template>
  <div class="app-update" :class="{ 'app-update--compact': compact }">
    <div class="app-update__row">
      <span class="app-update__version">{{ updates.state.currentVersion }}（{{ updates.statusText }}）</span>
      <div class="app-update__actions">
        <button type="button" class="pc__btn" :disabled="updates.working" @click="updates.check()">检查更新</button>
        <button v-if="updates.state.phase === 'ready'" type="button" class="pc__btn pc__btn--primary" :disabled="updates.working" @click="confirming = true">安装并重启</button>
        <button v-else-if="updates.state.status === 'available' && updates.state.canInstall" type="button" class="pc__btn pc__btn--primary" :disabled="updates.working" @click="updates.download()">下载更新</button>
        <button v-else-if="updates.state.status === 'available'" type="button" class="pc__btn pc__btn--primary" :disabled="updates.working" @click="openRelease">前往更新</button>
      </div>
    </div>
    <p v-if="updates.state.checkedAt && !compact" class="app-update__hint">上次检查：{{ new Date(updates.state.checkedAt).toLocaleString() }}</p>
    <p v-if="updates.state.status === 'available' && !updates.state.canInstall" class="app-update__hint">此版本未提供应用内更新包，请前往发布页下载。</p>
    <p v-if="updates.error || updates.state.error || linkError" class="app-update__error" role="alert">{{ updates.error || updates.state.error || linkError }}</p>
    <div v-if="confirming" class="app-update__confirm" role="alertdialog" aria-modal="true" aria-labelledby="update-confirm-title" @keydown.esc="confirming = false" @keydown.tab="trapFocus">
      <div class="app-update__dialog">
        <h3 id="update-confirm-title">安装主程序更新</h3>
        <p>安装会关闭 MyTools 并重新启动。请先保存所有插件中的编辑内容。</p>
        <div class="app-update__actions">
          <button v-focus type="button" class="pc__btn" @click="confirming = false" @keydown.esc="confirming = false">暂不安装</button>
          <button type="button" class="pc__btn pc__btn--primary" @click="install">已保存，安装并重启</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped src="../views/plugin-center/plugin-center.css"></style>

<style scoped>
.app-update { min-width:0; }
.app-update__row { display:flex; flex-wrap:wrap; align-items:center; gap:12px; }
.app-update__version { color:var(--panel-fg); font-size:13px; line-height:1.6; }
.app-update__actions { display:flex; flex-wrap:wrap; justify-content:flex-end; gap:8px; margin-left:auto; }
.app-update__hint { margin:8px 0 0; color:var(--text-dim); font-size:12px; }
.app-update__error { margin:8px 0 0; color:var(--error,#e06c75); font-size:12px; overflow-wrap:anywhere; }
.app-update__confirm { position:fixed; z-index:10000; inset:0; display:flex; align-items:center; justify-content:center; padding:24px; background:#0008; }
.app-update__dialog { width:min(440px,100%); padding:24px; border:1px solid var(--panel-control-border); border-radius:12px; color:var(--panel-fg); background:var(--surface-bg); box-shadow:0 16px 50px #0005; }
.app-update__dialog h3 { margin:0 0 14px; }
.app-update__dialog p { font-size:13px; line-height:1.7; margin:0 0 22px; }
</style>
