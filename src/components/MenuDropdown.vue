<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useActivePluginStore } from "@/stores/activePlugin";
import { useSearchStore } from "@/stores/search";
import { usePluginsStore } from "@/stores/plugins";
import { safeInvoke, tauriAvailable } from "@/lib/tauri";

const emit = defineEmits<{
  close: [];
}>();

const route = useRoute();
const router = useRouter();
const active = useActivePluginStore();
const search = useSearchStore();
const plugins = usePluginsStore();

const rootRef = ref<HTMLElement | null>(null);
const detaching = ref(false);

const detachEnabled = computed(() => {
  if (route.path === "/plugins") return true;
  if (route.name === "launcher") {
    const item = search.results[search.selectedIndex];
    return item?.kind === "plugin";
  }
  if (route.name === "plugin" && active.plugin) return true;
  return false;
});

const detachTitle = computed(() => {
  if (detachEnabled.value) return "分离当前内容到新窗口";
  return "当前页面没有可分离的内容";
});

function onBackToLauncher() {
  active.clear();
  void router.push({ name: "launcher" });
}

function gotoPlugins() {
  void router.push({ path: "/plugins", query: { section: "store" } });
  emit("close");
}

function gotoSettings() {
  void router.push({ path: "/plugins", query: { section: "settings" } });
  emit("close");
}

async function onDetach() {
  if (!detachEnabled.value) return;
  detaching.value = true;
  try {
    if (route.path === "/plugins") {
      await safeInvoke("plugin_center_open_window", {section: route.query.section || "store"});
      search.reset();
      void router.push({name: "launcher"});
    } else if (route.name === "launcher") {
      const item = search.results[search.selectedIndex];
      if (!item || item.kind !== "plugin") return;
      const [pluginId, feature] = item.id.split(":");
      if (!pluginId || !feature) return;
      await safeInvoke<string>("plugin_open_window", { pluginId, feature });
      search.reset();
    } else if (route.name === "plugin" && active.plugin) {
      await safeInvoke<string>("plugin_open_window", {
        pluginId: active.plugin.id,
        feature: active.plugin.code,
      });
      search.reset();
      active.clear();
      void router.push({ name: "launcher" });
    }
  } catch (e) {
    console.error("[MyTools] detach failed:", e);
  } finally {
    detaching.value = false;
    emit("close");
  }
}

async function onExit() {
  if (!tauriAvailable()) {
    emit("close");
    return;
  }
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  } catch {
    /* ignore */
  }
  emit("close");
}

function onClickOutside(e: MouseEvent) {
  if (!rootRef.value) return;
  if (!rootRef.value.contains(e.target as Node)) {
    emit("close");
  }
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    emit("close");
  }
}

onMounted(() => {
  setTimeout(() => document.addEventListener("click", onClickOutside), 0);
  document.addEventListener("keydown", onKeydown);
});

onBeforeUnmount(() => {
  document.removeEventListener("click", onClickOutside);
  document.removeEventListener("keydown", onKeydown);
});

void plugins;
</script>

<template>
  <div ref="rootRef" class="md" role="menu">
    <div v-if="active.plugin" class="md__section">
      <div class="md__plugin-info">
        <span class="md__plugin-icon">🔌</span>
        <span class="md__plugin-name">当前插件：{{ active.plugin.title }}</span>
      </div>
      <button class="md__item" type="button" role="menuitem" @click="onBackToLauncher">
        <span class="md__item-label">返回主页</span>
        <span class="md__item-hint">清空当前插件</span>
      </button>
      <div class="md__divider"></div>
    </div>

    <div class="md__section">
      <button class="md__item" type="button" role="menuitem" @click="safeInvoke('panel_show'); emit('close')"><span class="md__item-label">我的面板</span><span class="md__item-hint">鼠标中键</span></button>
      <button class="md__item" type="button" role="menuitem" @click="gotoPlugins">
        <span class="md__item-label">插件商店</span>
      </button>
      <button class="md__item" type="button" role="menuitem" @click="gotoSettings">
        <span class="md__item-label">偏好设置</span>
      </button>
      <div class="md__divider"></div>
    </div>

    <div class="md__section">
      <button
        class="md__item"
        type="button"
        role="menuitem"
        :disabled="!detachEnabled || detaching"
        :title="detachTitle"
        @click="onDetach"
      >
        <span class="md__item-label">分离窗口</span>
        <span v-if="!detachEnabled" class="md__item-hint">无可分离</span>
      </button>
      <div class="md__divider"></div>
    </div>

    <div class="md__section">
      <button class="md__item md__item--danger" type="button" role="menuitem" @click="onExit">
        <span class="md__item-label">退出</span>
        <span class="md__item-hint">关闭主窗口</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.md {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  min-width: 220px;
  padding: 6px;
  background: var(--panel-bg-strong);
  border: 1px solid var(--panel-control-border);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
  z-index: 100;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.md__section {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.md__plugin-info {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  font-size: 12px;
  color: var(--text-dim-strong);
  background: var(--hover-bg);
  border-radius: 6px;
  margin-bottom: 2px;
}

.md__plugin-icon {
  font-size: 14px;
}

.md__plugin-name {
  font-weight: 600;
  color: var(--accent);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 180px;
}

.md__divider {
  height: 1px;
  background: var(--panel-control-border);
  margin: 4px 0;
}

.md__item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 6px 10px;
  border: none;
  background: transparent;
  color: var(--panel-fg);
  font-size: 12px;
  cursor: pointer;
  border-radius: 6px;
  text-align: left;
}

.md__item:hover:not(:disabled) {
  background: var(--hover-bg);
}

.md__item:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.md__item-label {
  flex: 1;
}

.md__item-hint {
  font-size: 11px;
  color: var(--text-dim-strong);
}

.md__item--danger:hover:not(:disabled) {
  background: rgba(224, 108, 117, 0.12);
  color: #e06c75;
}
</style>
