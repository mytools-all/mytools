<script setup lang="ts">
import { onMounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { tauriAvailable } from "@/lib/tauri";
import { usePluginsStore } from "@/stores/plugins";
const plugins = usePluginsStore();
const busy = ref(false);
const npmSpec = ref("");
const npmBusy = ref(false);
const actionError = ref<string | null>(null);
const notice = ref<string | null>(null);
onMounted(() => { if (!plugins.root) void plugins.loadRoot(); });


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
</script>

<template>
  <section class="pc__page">
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
        <p v-if="plugins.root" class="pc__root" :title="plugins.root">{{ plugins.root }}</p>
      </div>
      <div class="pc__dev-block">
        <h4 class="pc__side-h">开发插件</h4>
        <p class="pc__dev-text">
          插件是一个 HTML 应用目录（可选 plugin.json / package.json manifest）。把构建产物放到插件根目录的子目录（如<code>mytools-demo</code>），点击「重新扫描」即可在「已安装」中看到。插件通过 <code>window.mytools.call(能力, 参数)</code> 调用宿主能力，更多协议见 <code>docs/04-plugin-spec.md</code>。
        </p>
      </div>
      <p v-if="actionError" class="pc__error">{{ actionError }}</p>
      <p v-else-if="notice" class="pc__notice">{{ notice }}</p>
    </div>
  </section>
</template>

<style scoped src="./plugin-center.css"></style>

<style scoped>
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

.pc__dev-text {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim);
  line-height: 1.6;
  /* max-width: 60ch; */
}

.pc__dev-text code {
  background: var(--chip-bg);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  color: var(--accent);
}
</style>
