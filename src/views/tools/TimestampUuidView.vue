<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useIntervalFn } from "@vueuse/core";
import ToolShell from "@/views/ToolShell.vue";

function uuidv4(): string {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

function isoToTimestamp(iso: string): number | null {
  if (!iso) return null;
  const n = Date.parse(iso);
  return Number.isFinite(n) ? n : null;
}

function timestampToIso(ts: number): string {
  return new Date(ts).toISOString();
}

const now = ref(0);

const { resume: startClock, pause: stopClock } = useIntervalFn(
  () => {
    now.value = Date.now();
  },
  1000,
  { immediate: false },
);

onMounted(() => {
  now.value = Date.now();
  startClock();
});

onUnmounted(() => {
  stopClock();
});

const ms = computed(() => now.value);
const iso = computed(() => new Date(now.value).toISOString());
const localStr = computed(() =>
  new Date(now.value).toLocaleString("zh-CN", { hour12: false }),
);

const tsInput = ref(String(Date.now()));
const isoOutput = computed(() => {
  const n = Number(tsInput.value);
  if (!Number.isFinite(n)) return "请输入合法的毫秒时间戳";
  return timestampToIso(n);
});

const isoInput = ref(new Date().toISOString().slice(0, 16));
const tsOutput = computed(() => {
  const n = isoToTimestamp(isoInput.value);
  return n === null ? "请输入合法的日期字符串" : String(n);
});

const uuids = ref<string[]>([]);
function generate() {
  uuids.value = Array.from({ length: 5 }, () => uuidv4());
}

async function copy(text: string) {
  await navigator.clipboard?.writeText(text).catch(() => {});
}
</script>

<template>
  <ToolShell>
    <template #title>时间戳 ⇄ 日期 / UUID</template>
    <div class="tu">
      <section class="tu__section">
        <h3 class="tu__h3">当前时间</h3>
        <div class="tu__grid">
          <div class="tu__cell">
            <span class="tu__label">毫秒</span>
            <code class="tu__value">{{ ms }}</code>
            <button class="tu__btn tu__btn--ghost" @click="copy(String(ms))">
              复制
            </button>
          </div>
          <div class="tu__cell">
            <span class="tu__label">ISO</span>
            <code class="tu__value">{{ iso }}</code>
            <button class="tu__btn tu__btn--ghost" @click="copy(iso)">
              复制
            </button>
          </div>
          <div class="tu__cell">
            <span class="tu__label">本地</span>
            <code class="tu__value">{{ localStr }}</code>
            <button class="tu__btn tu__btn--ghost" @click="copy(localStr)">
              复制
            </button>
          </div>
        </div>
      </section>

      <section class="tu__section">
        <h3 class="tu__h3">毫秒 ⇄ 日期</h3>
        <div class="tu__row">
          <div class="tu__cell">
            <span class="tu__label">毫秒</span>
            <input v-model="tsInput" class="tu__input" />
            <span class="tu__label">ISO</span>
            <code class="tu__value">{{ isoOutput }}</code>
            <button class="tu__btn tu__btn--ghost" @click="copy(isoOutput)">
              复制 ISO
            </button>
          </div>
          <div class="tu__cell">
            <span class="tu__label">日期</span>
            <input v-model="isoInput" class="tu__input" placeholder="2026-09-18T10:30" />
            <span class="tu__label">毫秒</span>
            <code class="tu__value">{{ tsOutput }}</code>
            <button class="tu__btn tu__btn--ghost" @click="copy(String(tsOutput))">
              复制毫秒
            </button>
          </div>
        </div>
      </section>

      <section class="tu__section">
        <h3 class="tu__h3">UUID v4</h3>
        <div class="tu__bar">
          <button class="tu__btn" @click="generate">生成 5 个</button>
          <button
            v-if="uuids.length"
            class="tu__btn tu__btn--ghost"
            @click="copy(uuids.join('\n'))"
          >
            复制全部
          </button>
        </div>
        <ul class="tu__uuids">
          <li v-for="u in uuids" :key="u" class="tu__uuid">
            <code>{{ u }}</code>
            <button class="tu__btn tu__btn--ghost" @click="copy(u)">复制</button>
          </li>
        </ul>
      </section>
    </div>
  </ToolShell>
</template>

<style scoped>
.tu {
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}

.tu__section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.tu__h3 {
  margin: 0;
  font-size: 11px;
  color: rgba(255, 255, 255, 0.55);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 600;
}

.tu__grid,
.tu__row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.tu__cell {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 8px;
  padding: 8px 10px;
}

.tu__label {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.5);
  text-transform: uppercase;
  letter-spacing: 0.4px;
}

.tu__value,
.tu__uuid code {
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  font-size: 12px;
  color: #6cf;
  word-break: break-all;
}

.tu__input {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 4px;
  padding: 4px 8px;
  color: inherit;
  font: inherit;
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  font-size: 12px;
  outline: none;
  min-width: 180px;
}

.tu__btn {
  background: rgba(102, 204, 255, 0.12);
  border: 1px solid rgba(102, 204, 255, 0.3);
  color: #6cf;
  padding: 4px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 11px;
}

.tu__btn--ghost {
  background: transparent;
  border-color: rgba(255, 255, 255, 0.12);
  color: rgba(255, 255, 255, 0.65);
}

.tu__bar {
  display: flex;
  gap: 6px;
}

.tu__uuids {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.tu__uuid {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  background: rgba(255, 255, 255, 0.03);
  border-radius: 6px;
}

@media (prefers-color-scheme: light) {
  .tu__cell {
    background: rgba(0, 0, 0, 0.02);
    border-color: rgba(0, 0, 0, 0.08);
  }
  .tu__btn--ghost {
    border-color: rgba(0, 0, 0, 0.15);
    color: rgba(0, 0, 0, 0.65);
  }
  .tu__input {
    background: rgba(0, 0, 0, 0.04);
    border-color: rgba(0, 0, 0, 0.1);
  }
  .tu__uuid {
    background: rgba(0, 0, 0, 0.02);
  }
}
</style>