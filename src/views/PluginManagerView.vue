<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import PluginDetailDrawer from "@/components/PluginDetailDrawer.vue";
import PluginStorePage from "./plugin-center/PluginStorePage.vue";
import InstalledPluginsPage from "./plugin-center/InstalledPluginsPage.vue";
import PluginDeveloperPage from "./plugin-center/PluginDeveloperPage.vue";
import PluginPreferencesPage from "./plugin-center/PluginPreferencesPage.vue";
import { usePluginsStore } from "@/stores/plugins";
import { useSettingsStore } from "@/stores/settings";
import { safeInvoke, tauriAvailable, isDetachedCenter } from "@/lib/tauri";

type Section = "store" | "installed" | "dev" | "settings";
const sections: { id: Section; label: string }[] = [
  {id:"store", label:"插件商店"},
  {id:"installed", label:"已安装"},
  {id:"dev", label:"开发者"},
  {id:"settings", label:"偏好设置"},
];
const route = useRoute();
const router = useRouter();
const plugins = usePluginsStore();
const settings = useSettingsStore();
const detached = isDetachedCenter();
const section = computed<Section>(() => {
  const value = route.query.section;
  return value === "installed" || value === "dev" || value === "settings" ? value : "store";
});
const detailName = computed(() => (section.value === "store" || section.value === "installed") && typeof route.query.detail === "string" ? route.query.detail : "");
function navTo(next: Section) { void router.push({path:"/plugins", query:{section:next}}); }
function showDetail(name: string) { void router.push({path:"/plugins",query:{...route.query,section:section.value,detail:name}}); }
function closeDetail() { const query={...route.query}; delete query.detail; void router.replace({path:"/plugins",query}); }
function layout(width: number, height: number) {
  if (!tauriAvailable() || detached) return;
  void safeInvoke("window_set_layout", {resizable:true,decorations:false,width,height});
}
onMounted(() => {
  layout(1024,760);
  void Promise.allSettled([settings.init(), plugins.ensureLoaded()]);
});
onBeforeUnmount(() => { layout(720,480); });
</script>

<template>
  <div class="pc" :inert="detailName ? true : undefined" :aria-hidden="detailName ? true : undefined">
    <aside class="pc__side">
      <nav class="pc__side-nav" aria-label="插件中心导航">
        <button v-for="item in sections" :key="item.id" type="button" class="pc__side-nav-item" :class="{'pc__side-nav-item--active':section === item.id}" :data-section="item.id" @click="navTo(item.id)">{{ item.label }}</button>
      </nav>
    </aside>
    <main class="pc__main">
      <KeepAlive>
        <PluginStorePage v-if="section === 'store'" :detached="detached" @detail="showDetail" />
        <InstalledPluginsPage v-else-if="section === 'installed'" :detached="detached" @detail="showDetail" />
        <PluginDeveloperPage v-else-if="section === 'dev'" />
        <PluginPreferencesPage v-else :detached="detached" />
      </KeepAlive>
    </main>
    <Teleport to="body"><Transition name="detail-drawer" appear><PluginDetailDrawer v-if="detailName" :key="detailName" :name="detailName" @close="closeDetail" /></Transition></Teleport>
  </div>
</template>

<style scoped>
.pc {
  background: var(--panel-bg);
  flex: 1;
  display: flex;
  gap: 18px;
  overflow: hidden;
  padding: 18px;
  height: 100%;
}

.pc__side {
  flex: none;
  width: 126px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.pc__side-nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.pc__side-nav-item {
  border: none;
  background: transparent;
  color: var(--text-dim);
  font-size: 13px;
  text-align: left;
  padding: 10px 12px;
  border-radius: 9px;
  cursor: pointer;
}

.pc__side-nav-item:hover {
  background: var(--hover-bg);
  color: var(--panel-fg);
}

.pc__side-nav-item--active {
  background: var(--accent-soft);
  color: var(--accent);
}

.pc__main {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow: hidden;
  min-width: 0;
  min-height: 0;
  position: relative;
}
</style>
