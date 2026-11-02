<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useActivePluginStore } from "@/stores/activePlugin";
import { usePluginsStore } from "@/stores/plugins";

const active = useActivePluginStore();
const plugins = usePluginsStore();
const route = useRoute();
const router = useRouter();

/**
 * The active-bar chip reflects whatever "sub-view" the user is inside:
 * a running plugin, the plugin store, or preferences. `×` always returns
 * to the launcher, mirroring Rubick's sub-input affordance.
 */
const chip = computed<{ icon: string; label: string; img?: string } | null>(() => {
  if (route.name === "plugin") {
    const a = active.plugin;
    if (!a) return null;
    const loaded = plugins.plugins.find((x) => x.manifest.name === a.id);
    return { icon: "🔌", label: a.title, img: loaded?.icon ?? a.icon };
  }
  if (route.path.startsWith("/plugins")) {
    if (route.query.section === "settings") {
      return { icon: "⚙", label: "偏好设置" };
    }
    if (route.query.section === "installed") {
      return { icon: "▣", label: "已安装插件" };
    }
    if (route.query.section === "dev") {
      return { icon: "⌥", label: "开发者" };
    }
    return { icon: "▦", label: "插件商店" };
  }
  return null;
});

function onClose() {
  active.clear();
  void router.push({ name: "launcher" });
}
</script>

<template>
  <div v-if="chip" class="pab" data-testid="plugin-active-bar">
    <span class="pab__icon" :title="chip.label">
      <img v-if="chip.img" :src="chip.img" :alt="chip.label" />
      <span v-else class="pab__icon-fallback">{{ chip.icon }}</span>
    </span>
    <span class="pab__name">{{ chip.label }}</span>
    <button
      type="button"
      class="pab__close"
      aria-label="返回启动器"
      title="返回启动器"
      @click="onClose"
    >
      ×
    </button>
  </div>
</template>

<style scoped>
.pab {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 4px 4px 4px 8px;
  border-radius: 999px;
  background: var(--chip-bg);
  border: 1px solid var(--panel-control-border);
  flex: none;
  max-width: 40%;
}

.pab__icon {
  width: 18px;
  height: 18px;
  border-radius: 4px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  background: var(--hover-bg);
  flex: none;
}

.pab__icon img {
  width: 18px;
  height: 18px;
  object-fit: contain;
}

.pab__icon-fallback {
  font-size: 12px;
}

.pab__name {
  font-size: 12px;
  font-weight: 500;
  color: var(--accent);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.pab__close {
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--text-dim-strong);
  border-radius: 999px;
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
  padding: 0;
  flex: none;
}

.pab__close:hover {
  background: var(--hover-bg);
  color: var(--panel-fg);
}
</style>