<script setup lang="ts">
import { ref, onBeforeUnmount } from "vue";
import { usePluginRuntime } from "@/runtime/usePluginRuntime";

defineOptions({ name: "PluginView" });
const { frameEl, iframeSrc, frameLoaded, fileBlocked, currentErrors, loadError, onFrameLoad } = usePluginRuntime();

const setFrame = (element: unknown) => { frameEl.value = element as HTMLIFrameElement | null; };

const errorsExpanded = ref(true);
const errorsCopied = ref(false);
let copyTimer: ReturnType<typeof setTimeout> | null = null;

async function copyErrors() {
  const text = currentErrors.value
    .map((e, i) => `[${i + 1}] ${e.kind}: ${e.message}\n${e.stack ?? ""}`)
    .join("\n\n");
  try {
    await navigator.clipboard.writeText(text);
    errorsCopied.value = true;
    if (copyTimer) clearTimeout(copyTimer);
    copyTimer = setTimeout(() => {
      errorsCopied.value = false;
      copyTimer = null;
    }, 1500);
  } catch {
    errorsCopied.value = false;
  }
}

onBeforeUnmount(() => {
  if (copyTimer) clearTimeout(copyTimer);
});
</script>

<template>
  <div class="pv">
    <div v-if="loadError" class="pv__loading" role="alert">{{ loadError }}</div>
    <div v-else-if="fileBlocked" class="pv__loading">
      插件资源服务器不可用，无法通过 file:// 加载插件页面，请重启应用后重试。
    </div>
    <div v-else-if="iframeSrc" class="pv__surface" :aria-busy="!frameLoaded">
      <iframe
        :key="iframeSrc"
        :class="{ 'pv__frame--loading': !frameLoaded }"
        :ref="setFrame"
        class="pv__frame"
        :src="iframeSrc"
        sandbox="allow-scripts allow-same-origin allow-forms"
        referrerpolicy="origin"
        title="plugin content"
        @load="onFrameLoad"
      />
      <div v-if="!frameLoaded" class="pv__loading pv__skeleton" role="status">
        <span class="pv__spinner" aria-hidden="true"></span>
        <span>正在打开工具…</span>
      </div>
    </div>
    <div v-else class="pv__loading">加载中…</div>
    <div
      v-if="currentErrors.length"
      class="pv__errors"
      :data-expanded="errorsExpanded"
    >
      <button
        class="pv__errors-toggle"
        type="button"
        @click="errorsExpanded = !errorsExpanded"
      >
        <span class="pv__errors-title">
          插件页错误 ({{ currentErrors.length }})
        </span>
        <span class="pv__errors-chev">{{ errorsExpanded ? "▾" : "▸" }}</span>
      </button>
      <div v-if="errorsExpanded" class="pv__errors-body">
        <button class="pv__errors-copy" type="button" @click="copyErrors">
          {{ errorsCopied ? "已复制" : "复制全部" }}
        </button>
        <ol class="pv__errors-list">
          <li
            v-for="(e, i) in currentErrors"
            :key="`${e.at}-${i}`"
            class="pv__errors-item"
          >
            <div class="pv__errors-kind">{{ e.kind }}</div>
            <div class="pv__errors-msg">{{ e.message }}</div>
            <pre v-if="e.stack" class="pv__errors-stack">{{ e.stack }}</pre>
          </li>
        </ol>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pv {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 10px;
}

.pv__loading {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.45);
  font-size: 12px;
}

.pv__surface { flex: 1; position: relative; min-height: 0; border-radius: 10px; overflow: hidden; background: var(--surface-bg); }
.pv__skeleton { position: absolute; inset: 0; gap: 12px; background: var(--surface-bg); }
.pv__spinner { width: 20px; height: 20px; border: 2px solid var(--panel-control-border); border-top-color: var(--accent); border-radius: 50%; animation: spin 800ms linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.pv__frame--loading { opacity: 0; pointer-events: none; }
.pv__frame {
  position: absolute;
  inset: 0;
  height: 100%;
  width: 100%;
  border: 0;
  background: white;
  border-radius: 8px;
}

.pv__skeleton {
  animation: pv-pulse 1.2s ease-in-out infinite;
}

@keyframes pv-pulse {
  0%,
  100% {
    opacity: 0.4;
  }
  50% {
    opacity: 0.75;
  }
}

.pv__errors {
  flex: 0 1 auto;
  min-height: 72px;
  margin-top: 8px;
  background: rgba(127, 29, 29, 0.65);
  border: 1px solid rgba(252, 165, 165, 0.5);
  border-radius: 8px;
  color: #fff5f5;
  font-size: 13px;
  max-height: 40vh;
  overflow: auto;
}

.pv__errors-toggle {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: rgba(252, 165, 165, 0.2);
  border: 0;
  padding: 8px 12px;
  color: inherit;
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
}

.pv__errors-title {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.pv__errors-title::before {
  content: "⚠";
  font-size: 14px;
  color: #fecaca;
}

.pv__errors-chev {
  opacity: 0.7;
  font-size: 12px;
}

.pv__errors-body {
  padding: 8px 12px 12px;
}

.pv__errors-copy {
  background: #fff5f5;
  color: #7f1d1d;
  border: 0;
  border-radius: 4px;
  padding: 3px 10px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  margin-bottom: 8px;
}

.pv__errors-list {
  list-style: decimal inside;
  margin: 0;
  padding: 0;
}

.pv__errors-item {
  margin: 0;
  padding: 8px 0;
  border-top: 1px solid rgba(252, 165, 165, 0.18);
}

.pv__errors-item:first-child {
  border-top: 0;
}

.pv__errors-kind {
  display: inline-block;
  background: #fecaca;
  color: #7f1d1d;
  border-radius: 4px;
  padding: 1px 6px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.4px;
}

.pv__errors-msg {
  margin-top: 4px;
  color: #fff5f5;
  font-weight: 600;
}

.pv__errors-stack {
  margin: 6px 0 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 11.5px;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-all;
  background: #1a0a0a;
  padding: 8px 10px;
  border-radius: 4px;
  color: #fda4af;
}
</style>
