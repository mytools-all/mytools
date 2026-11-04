<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import { RouterView, useRoute, useRouter } from "vue-router";
import FloatballView from "@/views/FloatballView.vue";
import WindowResizeHandles from "@/components/WindowResizeHandles.vue";
import PluginActiveBar from "@/components/PluginActiveBar.vue";
import MenuDropdown from "@/components/MenuDropdown.vue";
import SearchBox from "@/components/SearchBox.vue";
import appIcon from "../src-tauri/icons/128x128.png";
import { useActivePluginStore } from "@/stores/activePlugin";
import type { SearchResult } from "@/stores/search";
import { useResultActions } from "@/composables/useResultActions";
import { useAppShell } from "@/composables/useAppShell";
import CapabilityConsentDialog from "@/components/CapabilityConsentDialog.vue";

const active = useActivePluginStore();
const router = useRouter();
const route = useRoute();
const { run } = useResultActions();
const { isFloatballWindow, isPluginWindow, isPanelWindow, isCenterWindow } = useAppShell();

const menuOpen = ref(false);

function toggleMenu() {
  menuOpen.value = !menuOpen.value;
}

function closeMenu() {
  menuOpen.value = false;
}

async function onSelect(item: SearchResult) {
  await run(item);
}

function onBack() {
  if (route.name !== "plugin" && !route.path.startsWith("/plugins")) return;
  active.clear();
  void router.push({ name: "launcher" });
}

watch(
  () => route.path,
  () => {
    menuOpen.value = false;
  },
);

onBeforeUnmount(() => {
  menuOpen.value = false;
});
</script>

<template>
  <FloatballView v-if="isFloatballWindow" />
  <RouterView v-else-if="isPluginWindow || isPanelWindow || isCenterWindow" />
  <div v-else class="app-root">
    <header class="app-header" data-tauri-drag-region="deep">
      <img
        class="app-header__logo"
        data-testid="app-logo"
        :src="appIcon"
        alt="MyTools"
      />
      <PluginActiveBar />
      <SearchBox @select="onSelect" @back="onBack" />
      <div class="app-header__menu-wrap">
        <button
          type="button"
          class="app-header__menu-btn"
          :aria-label="menuOpen ? '关闭菜单' : '打开菜单'"
          :title="menuOpen ? '关闭菜单' : '菜单'"
          data-testid="app-menu-btn"
          @click="toggleMenu"
        >
          ⋮
        </button>
        <MenuDropdown v-if="menuOpen" @close="closeMenu" />
      </div>
    </header>
    <main class="app-main">
      <RouterView />
    </main>
    <WindowResizeHandles />
  </div>
  <CapabilityConsentDialog />
</template>

<style>
.app-root {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--panel-bg);
  color: var(--panel-fg);
  font-family:
    -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC",
    "Microsoft YaHei", sans-serif;
  border-radius: 14px;
  box-shadow: inset 0 0 0 1px var(--panel-control-border);
  overflow: hidden;
}

.app-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 18px;
  background: var(--panel-bg-strong);
  border-bottom: 1px solid var(--panel-border);
  flex: none;
}

.app-header__logo {
  width: 24px;
  height: 24px;
  border-radius: 5px;
  flex: none;
}

.app-header__spacer {
  flex: 1;
}

.app-header__menu-wrap {
  position: relative;
  flex: none;
}

.app-header__menu-btn {
  width: 32px;
  height: 36px;
  border-radius: 6px;
  border: 1px solid var(--panel-control-border);
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  font-size: 16px;
  line-height: 1;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.app-header__menu-btn:hover {
  background: var(--hover-bg);
  color: var(--panel-fg);
}

.app-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
}

</style>
