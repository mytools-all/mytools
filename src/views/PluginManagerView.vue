<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { open } from "@tauri-apps/plugin-dialog";
import { pluginDisplayName } from "@/lib/pluginPresentation";
import TopTabNav from "@/components/TopTabNav.vue";
import ToolCatalogGrid from "@/components/ToolCatalogGrid.vue";
import PluginDetailDrawer from "@/components/PluginDetailDrawer.vue";
import {
  usePluginsStore,
  type LoadedPlugin,
  type PluginOrigin,
} from "@/stores/plugins";
import { useGrantsStore } from "@/stores/grants";
import { useDirGrantsStore } from "@/stores/dirGrants";
import { useUsageStore } from "@/stores/usage";
import { useSearchStore } from "@/stores/search";
import {
  useSettingsStore,
  SETTINGS_CATEGORIES,
  type SettingsCategory,
  type Theme,
} from "@/stores/settings";
import { safeInvoke, tauriAvailable } from "@/lib/tauri";
import { fallbackIconDataUri } from "@/lib/iconFallback";

const PLUGIN_CENTER_WIDTH = 1024.0;
const PLUGIN_CENTER_HEIGHT = 760.0;
const PANEL_WIDTH = 720.0;
const PANEL_HEIGHT = 480.0;

type Section = "store" | "installed" | "dev" | "settings";

const plugins = usePluginsStore();
const grants = useGrantsStore();
const dirGrants = useDirGrantsStore();
const usage = useUsageStore();
const search = useSearchStore();
const settings = useSettingsStore();
const route = useRoute();
const router = useRouter();

const section = computed<Section>(() => {
  const s = route.query.section;
  if (s === "installed" || s === "dev" || s === "settings") return s;
  return "store";
});

const detailName = computed(() => (section.value === "store" || section.value === "installed") && typeof route.query.detail === "string" ? route.query.detail : "");
function showDetail(name: string) { void router.push({path:"/plugins",query:{...route.query,section:section.value,detail:name}}); }
function closeDetail(){const query={...route.query};delete query.detail;void router.replace({path:"/plugins",query});}

const enableFilter = ref("all");
const pending = ref<Record<string, boolean>>({});
const busy = ref(false);
const npmSpec = ref("");
const npmBusy = ref(false);
const actionError = ref<string | null>(null);
const notice = ref<string | null>(null);
const pluginRoot = ref("");
const rootNotice = ref<string | null>(null);
const newHotkey = ref(settings.hotkey);

const THEME_OPTIONS: { value: Theme; label: string; hint: string }[] = [
  { value: "system", label: "跟随系统", hint: "根据系统外观自动切换" },
  { value: "light", label: "浅色", hint: "固定使用浅色外观" },
  { value: "dark", label: "深色", hint: "固定使用深色外观" },
];

const pinnedIdSet = computed(() => new Set(usage.pinned));

const installTabs = computed(() => {
  const all = plugins.plugins.length;
  const enabled = plugins.enabledPlugins.length;
  return [
    { id: "all", label: "全部", count: all },
    { id: "enabled", label: "已启用", count: enabled, tone: "ok" as const },
    { id: "disabled", label: "已停用", count: all - enabled, tone: "err" as const },
  ];
});

const settingsTabs = computed(() =>
  SETTINGS_CATEGORIES.map((c) => ({
    id: c.key,
    label: c.label,
  })),
);

const installedPlugins = computed<LoadedPlugin[]>(() => {
  let xs = [...plugins.plugins];
  if (enableFilter.value === "enabled") {
    xs = xs.filter((p) => p.enabled);
  } else if (enableFilter.value === "disabled") {
    xs = xs.filter((p) => !p.enabled);
  }
  const q = search.query.trim().toLowerCase();
  if (q) {
    xs = xs.filter(
      (p) =>
        (p.manifest.pluginName ?? p.manifest.name).toLowerCase().includes(q) ||
        p.manifest.description.toLowerCase().includes(q) ||
        p.manifest.name.toLowerCase().includes(q),
    );
  }
  return xs;
});

/**
 * Settings are laid out as category panels rendered inline (no cards, no
 * drawer). The global search box maps a query to the first category whose
 * keywords match, so typing "主题" jumps straight to 外观.
 */
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

function pluginTitle(p: LoadedPlugin): string {
  return pluginDisplayName(p.manifest);
}

function originLabel(origin: PluginOrigin): string {
  if (origin === "official") return "官方";
  return "自定义";
}

function pluginPinLabel(name: string): string {
  return usage.isPinned(name) ? "取消固定" : "固定";
}

function onPin(name: string) {
  usage.togglePinned(name);
}

async function pickPluginRoot() {
  if (!tauriAvailable()) return;
  rootNotice.value = null;
  try {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string" && dir) {
      const applied = await plugins.setRoot(dir);
      if (applied) {
        pluginRoot.value = applied;
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
    pluginRoot.value = applied;
    rootNotice.value = "已恢复默认插件目录";
  }
}

function navTo(next: Section) {
  void router.push({ path: "/plugins", query: { section: next } });
}

async function toggle(p: LoadedPlugin) {
  pending.value[p.manifest.name] = true;
  actionError.value = null;
  try {
    await plugins.setEnabled(p.manifest.name, !p.enabled);
  } catch (e) {
    actionError.value = String(e);
  } finally {
    pending.value[p.manifest.name] = false;
  }
}

function openPlugin(p: LoadedPlugin) {
  const code = p.manifest.features?.[0]?.code ?? "default";
  usage.bump(p.manifest.name);
  void router.push({ name: "plugin", params: { id: p.manifest.name, code } });
}

async function reveal(p: LoadedPlugin) {
  pending.value[p.manifest.name] = true;
  actionError.value = null;
  try {
    await plugins.openDir(p.manifest.name);
  } catch (e) {
    actionError.value = String(e);
  } finally {
    pending.value[p.manifest.name] = false;
  }
}

async function remove(p: LoadedPlugin) {
  pending.value[p.manifest.name] = true;
  actionError.value = null;
  try {
    await plugins.uninstall(p.manifest.name);
  } catch (e) {
    actionError.value = String(e);
  } finally {
    pending.value[p.manifest.name] = false;
  }
}

async function installPack() {
  if (!tauriAvailable()) return;
  busy.value = true;
  actionError.value = null;
  notice.value = null;
  try {
    const file = await open({
      multiple: false,
      filters: [
        { name: "插件包", extensions: ["zip", "tgz", "gz"] },
        { name: "所有文件", extensions: ["*"] },
      ],
    });
    if (typeof file === "string" && file) {
      const name = await plugins.installPack(file);
      if (name) notice.value = `已安装插件 ${name}`;
    }
  } catch (e) {
    actionError.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function rescan() {
  busy.value = true;
  actionError.value = null;
  try {
    const n = await plugins.reload();
    notice.value = `插件已重新扫描（${n} 个）`;
  } catch (e) {
    actionError.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function installFromNpm() {
  const spec = npmSpec.value.trim();
  if (!spec) return;
  npmBusy.value = true;
  actionError.value = null;
  notice.value = null;
  try {
    const name = await plugins.installNpm(spec);
    if (name) {
      npmSpec.value = "";
      notice.value = `已从 npm 安装插件 ${name}`;
    }
  } catch (e) {
    actionError.value = String(e);
  } finally {
    npmBusy.value = false;
  }
}

function layout(width: number, height: number) {
  if (!tauriAvailable()) return;
  void safeInvoke("window_set_layout", {
    resizable: true,
    decorations: false,
    width,
    height,
  });
}

onMounted(async () => {
  if (tauriAvailable()) layout(PLUGIN_CENTER_WIDTH, PLUGIN_CENTER_HEIGHT);
  await Promise.allSettled([
    settings.init(),
    plugins.ensureLoaded(),
    grants.load(),
    dirGrants.load(),
    plugins.loadRoot().then((root) => { pluginRoot.value = root; }),
  ]);
  newHotkey.value = settings.hotkey;
});

onBeforeUnmount(() => {
  if (tauriAvailable()) layout(PANEL_WIDTH, PANEL_HEIGHT);
});
</script>

<template>
  <div class="pc" :inert="detailName ? true : undefined" :aria-hidden="detailName ? true : undefined">
    <aside class="pc__side">
      <nav class="pc__side-nav">
        <button
          type="button"
          class="pc__side-nav-item"
          :class="{ 'pc__side-nav-item--active': section === 'store' }"
          data-section="store"
          @click="navTo('store')"
        >
          插件商店
        </button>
        <button
          type="button"
          class="pc__side-nav-item"
          :class="{ 'pc__side-nav-item--active': section === 'installed' }"
          data-section="installed"
          @click="navTo('installed')"
        >
          已安装
        </button>
        <button
          type="button"
          class="pc__side-nav-item"
          :class="{ 'pc__side-nav-item--active': section === 'dev' }"
          data-section="dev"
          @click="navTo('dev')"
        >
          开发者
        </button>
        <button
          type="button"
          class="pc__side-nav-item"
          :class="{ 'pc__side-nav-item--active': section === 'settings' }"
          data-section="settings"
          @click="navTo('settings')"
        >
          偏好设置
        </button>
      </nav>
    </aside>

    <main class="pc__main">
      <template v-if="section === 'store'">
        <ToolCatalogGrid @detail="showDetail" />
      </template>

      <template v-else-if="section === 'installed'">
        <TopTabNav v-model="enableFilter" :tabs="installTabs" />
        <div v-if="plugins.plugins.length === 0 && !plugins.loading" class="pc__empty">
          暂无插件。切换到「开发者」安装工具包，或从本地目录安装。
        </div>
        <ul v-else class="pc__ilist">
          <li v-for="p in installedPlugins" :key="p.manifest.name" class="pc__iitem" role="button" tabindex="0" @click="showDetail(p.manifest.name)" @keydown.enter.self="showDetail(p.manifest.name)" @keydown.space.self.prevent="showDetail(p.manifest.name)">
            <span class="pc__iitem-icon">
              <img
                :src="p.icon || fallbackIconDataUri(pluginTitle(p))"
                :alt="pluginTitle(p)"
                class="pc__iitem-icon-img"
              />
            </span>
            <div class="pc__iitem-body">
              <div class="pc__iitem-title-row">
                <span class="pc__iitem-title">{{ pluginTitle(p) }}</span>
                <span class="pc__badge" :class="{ 'pc__badge--off': !p.enabled }">
                  {{ p.enabled ? "启用" : "停用" }}
                </span>
                <span class="pc__badge pc__badge--origin" :class="`pc__badge--${p.origin}`">
                  {{ originLabel(p.origin) }}
                </span>
                <span
                  v-if="p.incompatible"
                  class="pc__badge pc__badge--warn"
                  :title="p.incompatible"
                >
                  不兼容
                </span>
                <button
                  type="button"
                  class="pc__star"
                  :class="{ 'pc__star--on': pinnedIdSet.has(p.manifest.name) }"
                  :title="pluginPinLabel(p.manifest.name)"
                  :aria-label="pluginPinLabel(p.manifest.name)"
                  @click.stop="onPin(p.manifest.name)"
                >
                  {{ pinnedIdSet.has(p.manifest.name) ? "★" : "☆" }}
                </button>
              </div>
              <p class="pc__iitem-desc">{{ p.manifest.description }}</p>
              <p class="pc__iitem-meta">
                v{{ p.manifest.version }} · {{ p.manifest.name }}
              </p>
              <p v-if="p.incompatible" class="pc__iitem-warn">{{ p.incompatible }}</p>
</div>
            <div class="pc__iactions" @click.stop>
              <button
                class="pc__btn pc__btn--primary"
                :disabled="!p.enabled"
                @click="openPlugin(p)"
              >
                打开
              </button>
              <button class="pc__btn" :disabled="!!pending[p.manifest.name]" @click="toggle(p)">
                {{ p.enabled ? "停用" : "启用" }}
              </button>
              <button class="pc__btn" :disabled="!!pending[p.manifest.name]" @click="reveal(p)">
                打开目录
              </button>
              <button
                class="pc__btn pc__btn--danger"
                :disabled="!!pending[p.manifest.name]"
                @click="remove(p)"
              >
                卸载
              </button>
            </div>
          </li>
        </ul>
      </template>

      <template v-else-if="section === 'dev'">
        <p class="pc__dev-intro">
          插件是 HTML 应用目录：构建产物放入插件根目录的子目录，重新扫描即可加载。
        </p>
        <div class="pc__dev">
          <div class="pc__dev-block">
            <h4 class="pc__side-h">从 npm 安装</h4>
            <div class="pc__row">
              <input
                v-model="npmSpec"
                class="pc__search-input"
                placeholder="从 npm 安装插件包，如 mytools-some-plugin@1.0.0"
                @keyup.enter="installFromNpm"
              />
              <button
                class="pc__btn pc__btn--primary"
                :disabled="npmBusy || !npmSpec.trim()"
                @click="installFromNpm"
              >
                {{ npmBusy ? "下载中…" : "从 npm 安装" }}
              </button>
            </div>
          </div>
          <div class="pc__dev-block">
            <h4 class="pc__side-h">本地安装</h4>
            <div class="pc__row">
              <button class="pc__btn pc__btn--primary" :disabled="busy" @click="installPack">
                安装插件包（.zip / .tgz）
              </button>
              <button class="pc__btn" :disabled="busy" @click="rescan">重新扫描</button>
              <button class="pc__btn" @click="plugins.openRoot()">打开插件目录</button>
            </div>
            <p v-if="pluginRoot" class="pc__root" :title="pluginRoot">{{ pluginRoot }}</p>
          </div>
          <div class="pc__dev-block">
            <h4 class="pc__side-h">开发插件</h4>
            <p class="pc__dev-text">
              插件是一个 HTML 应用目录（可选 plugin.json / package.json
              manifest）。把构建产物放到插件根目录的子目录（如
              <code>mytools-demo</code>），点击「重新扫描」即可在「已安装」中看到。
              插件通过 <code>window.mytools.call(能力, 参数)</code> 调用宿主能力，
              更多协议见 <code>docs/04-plugin-spec.md</code>。
            </p>
          </div>
          <p v-if="actionError" class="pc__error">{{ actionError }}</p>
          <p v-else-if="notice" class="pc__notice">{{ notice }}</p>
        </div>
      </template>

      <template v-else-if="section === 'settings'">
        <TopTabNav
          :model-value="settings.settingsCategory"
          :tabs="settingsTabs"
          @update:model-value="settings.settingsCategory = $event as SettingsCategory"
        />

        <div class="pc__settings">
          <p v-if="search.query.trim()" class="pc__settings-hint">
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
                <span class="pc__hint">桌面右下角小球，拖动换位、单击唤起 launcher</span>
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
            <div class="pc__settings-block">
              <h4 class="pc__side-h">插件目录</h4>
              <div class="pc__row">
                <input class="pc__hotkey pc__root" :value="pluginRoot" readonly />
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
              <h4 class="pc__side-h">已加载插件</h4>
              <div class="pc__row">
                <span class="pc__text">共 {{ plugins.plugins.length }} 个插件</span>
                <button class="pc__btn" @click="rescan">重新扫描</button>
              </div>
            </div>

            <div class="pc__settings-block">
              <h4 class="pc__side-h">插件授权</h4>
              <p class="pc__hint">
                插件请求高权限能力时需要你确认；这里可以查看并撤销已授予的权限。
              </p>
              <div class="pc__row">
                <span class="pc__text">已授权 {{ grants.grants.length }} 项</span>
                <button class="pc__btn" :disabled="grants.loading" @click="grants.load()">
                  刷新
                </button>
              </div>
              <ul v-if="grants.grants.length > 0" class="pc__grant-list">
                <li
                  v-for="g in grants.grants"
                  :key="`${g.plugin}:${g.capability}`"
                  class="pc__grant-item"
                >
                  <span class="pc__text">{{ g.plugin }}</span>
                  <code class="pc__source">{{ g.capability }}</code>
                  <button
                    class="pc__btn pc__btn--danger"
                    @click="grants.revoke(g.plugin, g.capability)"
                  >
                    撤销
                  </button>
                </li>
              </ul>
              <p v-else-if="!grants.loading" class="pc__hint">暂无授权记录</p>
              <p v-if="grants.error" class="pc__message">{{ grants.error }}</p>
            </div>
          </template>

          <template v-else-if="settings.settingsCategory === 'data'">
            <div class="pc__settings-block">
              <h4 class="pc__side-h">目录授权</h4>
              <p class="pc__hint">
                插件访问真实文件目录需要你的授权：插件内触发「选择目录」后由系统对话框确认，
                这里可以随时撤销。
              </p>
              <div class="pc__row">
                <span class="pc__text">已授权 {{ dirGrants.grants.length }} 个目录</span>
                <button class="pc__btn" :disabled="dirGrants.loading" @click="dirGrants.load()">
                  刷新
                </button>
              </div>
              <ul v-if="dirGrants.grants.length > 0" class="pc__grant-list">
                <li
                  v-for="g in dirGrants.grants"
                  :key="`${g.plugin}:${g.path}`"
                  class="pc__grant-item"
                >
                  <span class="pc__text">{{ g.plugin }}</span>
                  <code class="pc__source" :title="g.path">{{ g.path }}</code>
                  <button
                    class="pc__btn pc__btn--danger"
                    @click="dirGrants.revoke(g.plugin, g.path)"
                  >
                    撤销
                  </button>
                </li>
              </ul>
              <p v-else-if="!dirGrants.loading" class="pc__hint">暂无目录授权</p>
              <p v-if="dirGrants.error" class="pc__message">{{ dirGrants.error }}</p>
            </div>
          </template>

          <template v-else>
            <div class="pc__settings-block">
              <h4 class="pc__side-h">关于</h4>
              <div class="pc__row">
                <span class="pc__text">
                  MyTools v0.1.0 · 轻量级插件化工具箱 · 仅 Windows
                </span>
              </div>
            </div>
          </template>
        </div>
      </template>
    </main>
    <Teleport to="body"><Transition name="detail-drawer" appear><PluginDetailDrawer v-if="detailName" :key="detailName" :name="detailName" @close="closeDetail" /></Transition></Teleport>
  </div>
</template>

<style scoped>
.pc {
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

.pc__row {
  display: flex;
  gap: 10px;
}

.pc__search-input {
  flex: 1;
  min-width: 0;
  padding: 8px 12px;
  border-radius: 8px;
  border: 1px solid var(--panel-control-border);
  background: transparent;
  color: var(--panel-fg);
  font-size: 13px;
  outline: none;
}

.pc__search-input:focus {
  border-color: var(--accent);
}

.pc__btn {
  padding: 8px 14px;
  border-radius: 8px;
  border: 1px solid var(--panel-control-border);
  background: transparent;
  color: var(--panel-fg);
  cursor: pointer;
  font-size: 13px;
  white-space: nowrap;
}

.pc__btn:hover:not(:disabled) {
  background: var(--hover-bg);
}

.pc__btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.pc__btn--primary {
  border-color: var(--accent);
  color: var(--accent);
}

.pc__btn--danger {
  border-color: rgba(224, 108, 117, 0.5);
  color: #e06c75;
}

.pc__settings {
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
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 18px;
  background: var(--surface-bg);
  border: 1px solid var(--panel-control-border);
  border-radius: 10px;
}

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

.pc__row--wrap {
  flex-wrap: wrap;
}

.pc__hotkey {
  flex: 1;
  max-width: 280px;
  padding: 8px 12px;
  border-radius: 8px;
  border: 1px solid var(--panel-control-border);
  background: transparent;
  color: var(--panel-fg);
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  font-size: 13px;
  outline: none;
}

.pc__hotkey:focus {
  border-color: var(--accent);
}

.pc__root {
  max-width: none;
  color: var(--text-dim);
}

.pc__source {
  flex: 1;
  min-width: 0;
  padding: 6px 10px;
  border-radius: 6px;
  background: var(--hover-bg);
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  font-size: 11px;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pc__text {
  font-size: 13px;
  color: var(--text-dim);
}

.pc__hint {
  font-size: 11px;
  color: var(--text-dim-strong);
  margin-left: auto;
}

.pc__status {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim-strong);
}

.pc__status--ok {
  color: var(--accent);
}

.pc__message {
  margin: 0;
  font-size: 12px;
  color: #e06c75;
}

.pc__link {
  background: transparent;
  border: none;
  color: var(--accent);
  font-size: 12px;
  cursor: pointer;
  padding: 0;
}

.pc__link:hover {
  text-decoration: underline;
}

.pc__empty {
  padding: 24px;
  text-align: center;
  font-size: 12px;
  color: var(--text-dim-strong);
  border: 1px dashed var(--panel-control-border);
  border-radius: 8px;
}

.pc__ilist {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
}

.pc__iitem:hover {border-color:var(--accent);}
.pc__iitem:focus-visible {outline:2px solid var(--accent);}
.pc__iitem {
  cursor:pointer;
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px 14px;
  border: 1px solid var(--panel-control-border);
  border-radius: 10px;
}

.pc__iitem-icon {
  width: 40px;
  height: 40px;
  flex: none;
  border-radius: 8px;
  overflow: hidden;
  background: var(--hover-bg);
}

.pc__iitem-icon-img {
  width: 40px;
  height: 40px;
  object-fit: contain;
}

.pc__iitem-body {
  flex: 1;
  min-width: 0;
}

.pc__iitem-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.pc__iitem-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--panel-fg);
}

.pc__badge {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 10px;
  border: 1px solid var(--accent);
  color: var(--accent);
}

.pc__badge--off {
  border-color: var(--text-dim-strong);
  color: var(--text-dim-strong);
}

.pc__badge--origin {
  border-color: var(--text-dim-strong);
  color: var(--text-dim-strong);
}

.pc__badge--official {
  border-color: var(--accent);
  color: var(--accent);
}

.pc__badge--warn {
  border-color: #d9534f;
  color: #d9534f;
}

.pc__iitem-warn {
  margin: 2px 0 0;
  font-size: 11px;
  color: #d9534f;
}

.pc__grant-list {
  list-style: none;
  margin: 6px 0 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.pc__grant-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  border-bottom: 1px solid var(--panel-border);
  font-size: 12px;
}

.pc__star {
  border: none;
  background: transparent;
  color: var(--text-dim-strong);
  font-size: 13px;
  cursor: pointer;
  line-height: 1;
  padding: 2px 4px;
  border-radius: 4px;
}

.pc__star:hover {
  color: #e5c07b;
}

.pc__star--on {
  color: #e5c07b;
}

.pc__iitem-desc {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pc__iitem-meta {
  margin: 2px 0 0;
  font-size: 11px;
  color: var(--text-dim-strong);
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
}

.pc__iactions {
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex: none;
}

.pc__error {
  margin: 0;
  font-size: 12px;
  color: #e06c75;
}

.pc__notice {
  margin: 0;
  font-size: 12px;
  color: var(--accent);
}

.pc__dev {
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}

.pc__dev-intro {
  margin: 0;
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--hover-bg);
  font-size: 12px;
  color: var(--text-dim);
  line-height: 1.6;
}

.pc__dev-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 14px;
  border: 1px solid var(--panel-control-border);
  border-radius: 10px;
}

.pc__side-h {
  margin: 0;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-dim-strong);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.pc__root {
  margin: 0;
  font-size: 11px;
  color: var(--text-dim-strong);
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pc__dev-text {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim);
  line-height: 1.6;
  max-width: 60ch;
}

.pc__dev-text code {
  background: var(--chip-bg);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  color: var(--accent);
}
</style>
