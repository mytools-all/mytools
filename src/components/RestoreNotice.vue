<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { safeInvoke } from "@/lib/tauri";
import { useSettingsStore } from "@/stores/settings";
const router = useRouter();
const settings = useSettingsStore();
const result = ref<{ at: string; success: boolean } | null>(null);
onMounted(async () => {
  const value = await safeInvoke<{ at: string; success: boolean } | null>("backup_last_result").catch(() => null);
  if (value && localStorage.getItem("mytools-restore-seen") !== value.at) result.value = value;
});
function dismiss() { if (result.value) localStorage.setItem("mytools-restore-seen", result.value.at); result.value = null; }
function details() { settings.settingsCategory = "backup"; void router.push({ path: "/plugins", query: { section: "settings" } }); dismiss(); }
</script>
<template><aside v-if="result" class="restore-notice" role="status"><span>{{ result.success ? '数据恢复已完成' : '上次数据恢复未完成，请查看结果' }}</span><button @click="details">查看详情</button><button aria-label="关闭恢复提示" @click="dismiss">×</button></aside></template>
<style scoped>.restore-notice{position:fixed;bottom:16px;left:16px;right:16px;z-index:9000;display:flex;gap:12px;align-items:center;padding:12px 16px;background:var(--surface-bg);color:var(--panel-fg);border:1px solid var(--panel-control-border);border-radius:8px;font-size:13px}.restore-notice span{flex:1}.restore-notice button{background:transparent;border:0;color:var(--accent);cursor:pointer}</style>
