<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import TopTabNav from "@/components/TopTabNav.vue";
import GrantBrowser from "@/components/GrantBrowser.vue";
import CatalogSourcesPanel from "./CatalogSourcesPanel.vue";
import AboutPanel from "./AboutPanel.vue";
import { usePluginsStore } from "@/stores/plugins";
import { useGrantsStore } from "@/stores/grants";
import { useDirGrantsStore } from "@/stores/dirGrants";
import { useCenterSearchStore } from "@/stores/centerSearch";
import { useSettingsStore, SETTINGS_CATEGORIES, type SettingsCategory, type Theme } from "@/stores/settings";
import { tauriAvailable } from "@/lib/tauri";

defineProps<{ detached: boolean }>();
const plugins = usePluginsStore();
const grants = useGrantsStore();
const dirGrants = useDirGrantsStore();
const search = useCenterSearchStore();
const settings = useSettingsStore();
const rootNotice = ref<string | null>(null);
const newHotkey = ref(settings.hotkey);
const scanBusy = ref(false);
const scanNotice = ref<string | null>(null);
const capabilityRows = computed(() => grants.grants.map(record => ({plugin:record.plugin,permission:record.capability})));
const directoryRows = computed(() => dirGrants.grants.map(record => ({plugin:record.plugin,permission:record.path})));
watch(() => settings.hotkey, (value, previous) => { if (newHotkey.value === previous) newHotkey.value = value; });
onMounted(() => { void Promise.allSettled([grants.load(), dirGrants.load(), ...(!plugins.root ? [plugins.loadRoot()] : [])]); });

const THEME_OPTIONS: { value: Theme; label: string; hint: string }[] = [
  { value: "system", label: "跟随系统", hint: "根据系统外观自动切换" },
  { value: "light", label: "浅色", hint: "固定使用浅色外观" },
  { value: "dark", label: "深色", hint: "固定使用深色外观" },
];
const settingsTabs = computed(() =>
  SETTINGS_CATEGORIES.map((c) => ({
    id: c.key,
    label: c.label,
  })),
);
const SETTINGS_KEYWORDS: Record<SettingsCategory, string[]> = {
  appearance: ["外观", "主题", "深色", "浅色", "托盘", "悬浮球", "theme", "tray", "floatball"],
  plugins: ["插件", "目录", "安装", "扫描", "plugin", "root", "install"],
  data: ["目录授权", "授权", "目录", "grant", "permission", "access"],
  shortcuts: ["快捷键", "唤起", "热键", "hotkey", "shortcut"],
  about: ["关于", "版本", "about", "version"],
};

function matchSettingsCategory(raw: string): SettingsCategory | null {
  const q = raw.trim().toLowerCase();
  if (!q) return null;
  for (const c of SETTINGS_CATEGORIES) {
    if (c.label.toLowerCase().includes(q)) return c.key;
  }
  for (const [key, words] of Object.entries(SETTINGS_KEYWORDS)) {
    if (words.some((w) => w.toLowerCase().includes(q))) return key as SettingsCategory;
  }
  return null;
}

function onSettingsSearchJump(value: string) {
  const hit = matchSettingsCategory(value);
  if (hit) settings.settingsCategory = hit;
}

async function pickPluginRoot() {
  if (!tauriAvailable()) return;
  rootNotice.value = null;
  try {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string" && dir) {
      const applied = await plugins.setRoot(dir);
      if (applied) {
        rootNotice.value = "插件目录已更新";
      }
    }
  } catch (e) {
    rootNotice.value = String(e);
  }
}

async function resetPluginRoot() {
  rootNotice.value = null;
  const applied = await plugins.resetRoot();
  if (applied) {
    rootNotice.value = "已恢复默认插件目录";
  }
}

async function rescan() {
  scanBusy.value = true;
  try {
    const count = await plugins.reload();
    scanNotice.value = plugins.error ? null : `插件已重新扫描（${count} 个）`;
  } finally { scanBusy.value = false; }
}
</script>

<template>
  <section class="pc__page">
    <TopTabNav
      :model-value="settings.settingsCategory"
      :tabs="settingsTabs"
      @update:model-value="settings.settingsCategory = $event as SettingsCategory"
    />

    <div class="pc__settings">
      <p v-if="!detached && search.query.trim()" class="pc__settings-hint">
        搜索 “{{ search.query }}” —
        <button class="pc__link" type="button" @click="onSettingsSearchJump(search.query)">
          跳到匹配分类
        </button>
      </p>

      <template v-if="settings.settingsCategory === 'appearance'">
        <div class="pc__settings-block">
          <h4 class="pc__side-h">主题</h4>
          <div class="pc__row pc__row--wrap">
            <button
              v-for="opt in THEME_OPTIONS"
              :key="opt.value"
              type="button"
              class="pc__choice"
              :class="{ 'pc__choice--active': settings.theme === opt.value }"
              @click="settings.setTheme(opt.value)"
            >
              <span class="pc__choice-label">{{ opt.label }}</span>
              <span class="pc__choice-hint">{{ opt.hint }}</span>
            </button>
          </div>
        </div>

        <div class="pc__settings-block">
          <h4 class="pc__side-h">常驻</h4>
          <label class="pc__settings-toggle">
            <input
              type="checkbox"
              class="pc__checkbox"
              :checked="settings.trayEnabled"
              @change="settings.setTrayEnabled(($event.target as HTMLInputElement).checked)"
            />
            <span class="pc__text">系统托盘</span>
            <span class="pc__hint">任务栏托盘图标，右键菜单快捷操作</span>
          </label>
          <label class="pc__settings-toggle">
            <input
              type="checkbox"
              class="pc__checkbox"
              :checked="settings.floatballEnabled"
              @change="settings.setFloatballEnabled(($event.target as HTMLInputElement).checked)"
            />
            <span class="pc__text">悬浮球</span>
            <span class="pc__hint">单击打开主页，右键打开我的面板，拖动调整位置</span>
          </label>
        </div>
      </template>

      <template v-else-if="settings.settingsCategory === 'shortcuts'">
        <div class="pc__settings-block">
          <h4 class="pc__side-h">唤起快捷键</h4>
          <div class="pc__row">
            <input
              v-model="newHotkey"
              class="pc__hotkey"
              placeholder="例如 Alt+Space、Ctrl+Shift+X"
            />
            <button
              class="pc__btn pc__btn--primary"
              :disabled="settings.saving"
              @click="settings.registerHotkey(newHotkey.trim())"
            >
              {{ settings.saving ? "注册中…" : "注册" }}
            </button>
            <button class="pc__btn" :disabled="settings.saving" @click="settings.unregisterHotkey()">
              注销
            </button>
          </div>
          <p class="pc__status" :class="{ 'pc__status--ok': settings.registered }">
            {{ settings.registered ? "快捷键已生效" : "快捷键未注册" }}
          </p>
          <p v-if="settings.message" class="pc__message">{{ settings.message }}</p>
        </div>
      </template>

      <template v-else-if="settings.settingsCategory === 'plugins'">
        <CatalogSourcesPanel />
        <div class="pc__settings-block">
          <h4 class="pc__side-h">插件目录</h4>
          <div class="pc__row">
            <input class="pc__hotkey pc__root" :value="plugins.root" readonly />
            <button class="pc__btn" :disabled="plugins.rootBusy" @click="pickPluginRoot">
              更改…
            </button>
            <button class="pc__btn" :disabled="plugins.rootBusy" @click="resetPluginRoot">
              恢复默认
            </button>
            <button class="pc__btn" @click="plugins.openRoot()">打开</button>
          </div>
          <p v-if="rootNotice" class="pc__status pc__status--ok">{{ rootNotice }}</p>
          <p v-if="plugins.error" class="pc__message">{{ plugins.error }}</p>
        </div>

        <div class="pc__settings-block">
          <div class="pc__section-head">
            <div><h4>已加载插件</h4><span class="pc__text">共 {{ plugins.plugins.length }} 个插件</span></div>
            <button class="pc__btn" :disabled="scanBusy" @click="rescan">重新扫描</button>
          </div>
        </div>

        <p v-if="scanNotice" class="pc__notice">{{ scanNotice }}</p>
        <GrantBrowser title="插件授权" description="查看插件已获准使用的能力，可按插件或权限分组并逐项撤销。" :records="capabilityRows" :loading="grants.loading" :error="grants.error" :revoke="grants.revoke" @refresh="grants.load()" />
      </template>

      <template v-else-if="settings.settingsCategory === 'data'">
        <GrantBrowser title="目录授权" description="按插件或授权目录查看目录访问权限。撤销后，插件再次访问需重新授权。" directory :records="directoryRows" :loading="dirGrants.loading" :error="dirGrants.error" :revoke="dirGrants.revoke" @refresh="dirGrants.load()" />
      </template>

      <template v-else>
        <AboutPanel />
      </template>
    </div>
  </section>
</template>

<style scoped src="./plugin-center.css"></style>

<style scoped>
.pc__settings {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
  padding-right: 4px;
  scrollbar-gutter: stable;
}

.pc__settings-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim-strong);
  display: flex;
  align-items: center;
  gap: 6px;
}

.pc__settings-block {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 18px;
  background: var(--surface-bg);
  border: 1px solid var(--panel-control-border);
  border-radius: 10px;
}

.pc__section-head,.pc__section-head>div { display:flex; align-items:center; gap:12px; }

.pc__section-head { justify-content:space-between; }

.pc__section-head h4 { margin:0; font-size:13px; font-weight:600; }

.pc__settings-block > .pc__hint { margin:0; line-height:1.6; }

.pc__settings-toggle {
  display: flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
}

.pc__checkbox {
  width: 16px;
  height: 16px;
  accent-color: var(--accent);
  cursor: pointer;
  flex: none;
}

.pc__choice {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 4px;
  min-width: 140px;
  padding: 10px 14px;
  border-radius: 8px;
  border: 1px solid var(--panel-control-border);
  background: transparent;
  color: var(--panel-fg);
  cursor: pointer;
  text-align: left;
}

.pc__choice:hover {
  background: var(--hover-bg);
}

.pc__choice--active {
  border-color: var(--accent);
  background: var(--chip-bg);
}

.pc__choice-label {
  font-size: 13px;
  font-weight: 600;
}

.pc__choice-hint {
  font-size: 11px;
  color: var(--text-dim-strong);
}
</style>
