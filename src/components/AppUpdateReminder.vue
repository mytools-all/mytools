<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useAppUpdatesStore } from "@/stores/appUpdates";
import { useSettingsStore } from "@/stores/settings";

const updates = useAppUpdatesStore();
const settings = useSettingsStore();
const router = useRouter();
const opening = ref(false);
onMounted(() => { void updates.initialize(); });
onBeforeUnmount(() => updates.dispose());
async function details() {
  settings.settingsCategory = "about";
  opening.value = true;
  try { await router.push({ name: "plugins", query: { section: "settings" } }); }
  finally { opening.value = false; }
}
</script>

<template>
  <aside v-if="updates.showReminder" class="update-reminder" aria-live="polite">
    <span>MyTools {{ updates.state.latestVersion }} {{ updates.state.phase === 'ready' ? '已下载，等待安装' : '可更新' }}</span>
    <button type="button" :disabled="opening" @click="details">查看更新</button>
    <button type="button" @click="updates.dismiss()">稍后提醒</button>
  </aside>
</template>

<style scoped>
.update-reminder { position:fixed; z-index:500; right:18px; bottom:18px; max-width:calc(100vw - 36px); display:flex; align-items:center; flex-wrap:wrap; gap:12px; padding:14px 16px; border:1px solid var(--panel-control-border); border-radius:10px; background:var(--surface-bg); color:var(--panel-fg); box-shadow:0 6px 24px #0003; font-size:12px; }
.update-reminder button { color:var(--accent); background:transparent; border:0; cursor:pointer; padding:4px; }
</style>
