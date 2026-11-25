<script setup lang="ts">
import { onMounted } from "vue";
import AppUpdateStatus from "./AppUpdateStatus.vue";
import { useAppUpdatesStore, type UpdateMode } from "@/stores/appUpdates";
defineProps<{section:"updates"|"startup"}>();

const updates = useAppUpdatesStore();
onMounted(() => { void updates.initialize().then(() => updates.refreshAutostart()); });
</script>

<template>
  <div v-if="section==='startup'" class="pc__settings-block">
    <h4 class="pc__side-h">启动</h4>
    <label class="pc__settings-toggle">
      <input class="pc__checkbox" type="checkbox" :checked="updates.autostart === true" :disabled="updates.savingAutostart || updates.autostart === null" @change="updates.setAutostart(($event.target as HTMLInputElement).checked)" />
      <span class="pc__text">开机自启</span>
      <span class="pc__hint">登录 Windows 后启动到托盘</span>
    </label>
    <p v-if="updates.autostartError" class="general-update__error" role="alert">{{ updates.autostartError }} <button class="pc__btn" @click="updates.refreshAutostart()">重试</button></p>
  </div>
  <div v-if="section==='updates'" class="pc__settings-block">
    <h4 class="pc__side-h">主程序更新</h4>
    <label class="general-update__mode">更新方式
      <select class="general-update__select" :value="updates.state.mode" :disabled="updates.savingMode" @change="updates.setMode(($event.target as HTMLSelectElement).value as UpdateMode)">
        <option value="disabled">禁止自动检查</option>
        <option value="notify">自动检查并提醒</option>
        <option value="download">自动检查并下载</option>
      </select>
    </label>
    <p class="general-update__hint">启动时检查一次。自动下载后仍需保存内容并确认安装；关闭自动检查后，也可手动检查。插件更新在“已安装”中管理。</p>
    <AppUpdateStatus />
  </div>
</template>

<style scoped src="../views/plugin-center/plugin-center.css"></style>

<style scoped>
.general-update__mode { display:flex; align-items:center; gap:14px; font-size:13px; color:var(--panel-fg); }
.general-update__select { min-width:200px; padding:8px 12px; border:1px solid var(--panel-control-border); border-radius:6px; background:var(--panel-control-bg); color:var(--panel-fg); color-scheme:inherit; }
.general-update__hint { font-size:12px; line-height:1.7; margin:12px 0 16px; color:var(--text-dim); }
.general-update__error { font-size:12px; color:var(--error,#e06c75); }
</style>
