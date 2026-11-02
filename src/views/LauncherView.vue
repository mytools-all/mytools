<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useRouter } from "vue-router";
import { useSearchStore, type SearchResult } from "@/stores/search";
import { usePluginsStore } from "@/stores/plugins";
import { useAppIndexStore } from "@/stores/appIndex";
import { useUsageStore, isAppPinKey, isPluginPinKey } from "@/stores/usage";
import { useResultActions } from "@/composables/useResultActions";
import { pluginDisplayName } from "@/lib/pluginPresentation";
import ResultList from "@/components/ResultList.vue";
import PluginTiles, { type TileItem } from "@/components/PluginTiles.vue";
import { usePanelStore } from "@/stores/panel";
import { safeInvoke } from "@/lib/tauri";

const search = useSearchStore();
const plugins = usePluginsStore();
const appIndex = useAppIndexStore();
const usage = useUsageStore();
const router = useRouter();
const panel = usePanelStore();
const panelIds = computed(() => new Set(plugins.plugins.filter(p => panel.hasPlugin(p.manifest.name, p.manifest.features[0]?.code)).map(p => p.manifest.name)));
function addToPanel(id: string) {
  const plugin = plugins.plugins.find(p => p.manifest.name === id);
  if (plugin) panel.addPlugin(plugin);
}
function removeFromPanel(id: string) { panel.entries = panel.entries.filter(e => e.kind !== "plugin" || e.target !== id); }
const { run: runResult, detachSelected: detachSelectedToWindow } = useResultActions();

const pinnedIdSet = computed(() => new Set(usage.pinned));

function guessAppName(target: string): string {
  // Legacy pins from the old `ms-settings:` table still launch fine; show
  // them as the Settings app instead of the raw URI.
  if (target.toLowerCase().startsWith("ms-settings:")) return "设置";
  const last = target.split(/[\\/]/).pop() || target;
  return last.replace(/\.lnk$/i, "").replace(/\.exe$/i, "") || target;
}

function guessAppSubtitle(target: string): string {
  return target.toLowerCase().startsWith("ms-settings:") ? "app (system)" : "app";
}

function getAppTileFromSearch(key: string): TileItem | null {
  const target = key.slice(4);
  const lowered = target.toLowerCase();
  // The index snapshot outlives `search.results`, so pinned apps still show
  // their real Chinese name after a restart or before any query is typed.
  const res = search.results.find((r) => r.kind === "app" && r.id.toLowerCase() === lowered);
  const entry = appIndex.find(target);
  return {
    id: key,
    name: entry?.name || res?.title || guessAppName(target),
    subtitle: entry?.subtitle || res?.subtitle || guessAppSubtitle(target),
    icon: appIndex.iconFor(target),
  };
}

onMounted(async () => {
  await plugins.ensureLoaded();
});

function toTile(name: string): TileItem | null {
  const p = plugins.plugins.find((x) => x.manifest.name === name);
  if (!p) return null;
  const feature = p.manifest.features[0];
  return {
    id: p.manifest.name,
    name: pluginDisplayName(p.manifest),
    subtitle: feature?.explain ?? p.manifest.description,
    icon: p.icon,
  };
}

const recentTiles = computed<TileItem[]>(() =>
  usage.recent
    .map((key) => {
      if (isPluginPinKey(key)) {
        if (!plugins.plugins.some((p) => p.manifest.name === key)) return null;
        return toTile(key);
      }
      if (isAppPinKey(key)) {
        return getAppTileFromSearch(key);
      }
      return null;
    })
    .filter((t): t is TileItem => t !== null)
    .slice(0, 12),
);

const recentFallbackTiles = computed<TileItem[]>(() => {
  if (recentTiles.value.length > 0) return recentTiles.value;
  const used = new Set(usage.recent);
  return plugins.enabledPlugins
    .filter((p) => !used.has(p.manifest.name))
    .slice(0, 12)
    .map((p) => toTile(p.manifest.name))
    .filter((t): t is TileItem => t !== null);
});

const pinnedTiles = computed<TileItem[]>(() =>
  usage.pinned
    .map((key) => {
      if (isPluginPinKey(key)) {
        if (!plugins.plugins.some((p) => p.manifest.name === key)) return null;
        return toTile(key);
      }
      if (isAppPinKey(key)) {
        return getAppTileFromSearch(key);
      }
      return null;
    })
    .filter((t): t is TileItem => t !== null),
);

watch(
  () => [...usage.pinned, ...usage.recent].filter(isAppPinKey),
  (keys) => { for (const key of new Set(keys)) void appIndex.ensureIcon(key.slice(4)); },
  { immediate: true },
);

function openPluginByName(name: string) {
  if (name.toLowerCase().startsWith("app:")) {
    const path = name.slice(4);
    void (async () => {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("app_launcher_open", { path });
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().hide();
      usage.bumpApp(name);
    })();
    return;
  }
  const p = plugins.plugins.find((x) => x.manifest.name === name);
  if (!p || p.manifest.features.length === 0) return;
  const feature = p.manifest.features[0];
  usage.bump(p.manifest.name);
  void router.push({
    name: "plugin",
    params: { id: p.manifest.name, code: feature.code },
  });
}

function onPinTile(id: string) {
  usage.togglePinned(id);
}

function onRemoveRecent(id: string) {
  usage.removeRecent(id);
}

function onPinResult(item: SearchResult) {
  if (item.kind === "app") {
    usage.togglePinnedApp(item.id);
    return;
  }
  usage.togglePinned(item.id.split(":")[0]);
}

async function onSelect(item: SearchResult) {
  await runResult(item);
}

defineExpose({ detachSelectedToWindow });
</script>

<template>
  <div class="launcher">
    <main class="launcher__main">
      <ResultList
        v-if="search.results.length"
        :results="search.results"
        :selected-index="search.selectedIndex"
        :pinned-ids="pinnedIdSet"
        @select="onSelect"
        @pin="onPinResult"
      />
      <div v-else-if="search.query.trim() !== ''" class="launcher__empty">
        <p v-if="search.error" class="launcher__error">
          搜索失败：<code>{{ search.error }}</code>
        </p>
        <p v-else-if="search.loading" class="launcher__hint">
          正在搜索 "<strong>{{ search.query }}</strong>"…
          <span class="launcher__dot">●</span>
        </p>
        <p v-else class="launcher__hint">
          没有找到 "<strong>{{ search.query }}</strong>" 的相关结果
        </p>
      </div>
      <div v-else class="launcher__home">
        <PluginTiles
          title="最近使用"
          panelable
          :panel-ids="panelIds"
          @panel="addToPanel"
          @unpanel="removeFromPanel"
          :items="recentFallbackTiles"
          removable
          pinnable
          pin-on-hover
          :pinned-ids="pinnedIdSet"
          empty-text="还没有使用记录；在插件中心安装插件后，从启动器打开会自动出现在这里。"
          @open="openPluginByName"
          @remove="onRemoveRecent"
          @pin="onPinTile"
        />
        <PluginTiles
          title="固定收藏"
          panelable
          :panel-ids="panelIds"
          @panel="addToPanel"
          @unpanel="removeFromPanel"
          :items="pinnedTiles"
          pinnable
          :pinned-ids="pinnedIdSet"
          empty-text="点击搜索结果行尾的 ☆（或插件中心卡片上的 ☆）固定插件。"
          @open="openPluginByName"
          @pin="onPinTile"
        />
      </div>
    </main>
    <footer class="launcher__footer">
      <button type="button" @click="safeInvoke('panel_show')">我的面板</button>
      <span v-if="search.query.trim()"><kbd>↑</kbd> <kbd>↓</kbd> 选择 <span class="launcher__separator">·</span> <kbd>↵</kbd> 打开</span>
      <span v-else>输入 <kbd>=</kbd> 即可快速计算</span>
      <button type="button" @click="router.push({ path: '/plugins', query: { section: 'store' } })">浏览工具 <span aria-hidden="true">↗</span></button>
    </footer>
  </div>
</template>

<style scoped>
.launcher {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.launcher__main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
  padding: 0 20px 10px;
}

.launcher__empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  gap: 10px;
  padding: 24px;
}

.launcher__home {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 20px;
  overflow-y: auto;
  padding: 18px 0 8px;
  scrollbar-gutter: stable;
}

.launcher__hint {
  color: var(--text-dim);
  font-size: 13px;
  margin: 0;
  text-align: center;
}

.launcher__hint strong {
  color: var(--accent);
  font-weight: 600;
}

.launcher__dot {
  color: var(--accent);
  animation: pulse 1s infinite;
  margin-left: 4px;
}

.launcher__error {
  color: var(--danger, #e5534b);
  font-size: 13px;
  margin: 0;
  text-align: center;
}

.launcher__error code {
  color: var(--text-dim);
  font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
  font-size: 12px;
  word-break: break-all;
}

@keyframes pulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 1; }
}
.launcher__footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 9px 20px;
  border-top: 1px solid var(--panel-border);
  color: var(--text-dim-strong);
  font-size: 11px;
  flex: none;
}
.launcher__footer button {
  border: 0;
  padding: 2px 0;
  color: var(--text-dim);
  background: transparent;
  cursor: pointer;
  font-size: 11px;
}
.launcher__footer button:hover { color: var(--accent); }
.launcher__separator { margin: 0 5px; }
.launcher__footer kbd { font: inherit; color: var(--text-dim); }
</style>
