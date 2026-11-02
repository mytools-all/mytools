<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import ToolShell from "@/views/ToolShell.vue";

interface ColorView {
  hex: string;
  rgb: string;
  r: number;
  g: number;
  b: number;
}

const hexInput = ref("#6cf");
const rgbInput = ref("rgb(102, 204, 255)");
const parsed = ref<ColorView | null>(null);
const error = ref<string | null>(null);

const previewStyle = computed(() => ({
  background: parsed.value?.hex ?? "transparent",
}));

const luminance = computed(() => {
  if (!parsed.value) return 1;
  const { r, g, b } = parsed.value;
  const lin = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
});

const textOnPreview = computed(() => (luminance.value > 0.5 ? "#1a1a1a" : "#f5f5f5"));

async function parseHex() {
  error.value = null;
  try {
    parsed.value = await invoke<ColorView>("color_parse_hex", { input: hexInput.value });
  } catch (e) {
    error.value = String(e);
    parsed.value = null;
  }
}

async function parseRgb() {
  error.value = null;
  try {
    parsed.value = await invoke<ColorView>("color_parse_rgb", { input: rgbInput.value });
  } catch (e) {
    error.value = String(e);
    parsed.value = null;
  }
}

async function copyHex() {
  if (!parsed.value) return;
  await navigator.clipboard?.writeText(parsed.value.hex).catch(() => {});
}

async function copyRgb() {
  if (!parsed.value) return;
  await navigator.clipboard?.writeText(parsed.value.rgb).catch(() => {});
}
</script>

<template>
  <ToolShell>
    <template #title>颜色选择 / 转换</template>
    <div class="color">
      <div class="color__form">
        <div class="color__field">
          <label class="color__label">HEX</label>
          <input
            v-model="hexInput"
            class="color__input"
            spellcheck="false"
            @keydown.enter="parseHex"
          />
          <button class="color__btn" @click="parseHex">解析</button>
        </div>
        <div class="color__field">
          <label class="color__label">RGB</label>
          <input
            v-model="rgbInput"
            class="color__input"
            spellcheck="false"
            @keydown.enter="parseRgb"
          />
          <button class="color__btn color__btn--ghost" @click="parseRgb">
            解析
          </button>
        </div>
      </div>

      <div v-if="error" class="color__error">{{ error }}</div>

      <div v-if="parsed" class="color__result">
        <div class="color__swatch" :style="previewStyle">
          <span class="color__hex" :style="{ color: textOnPreview }">{{ parsed.hex }}</span>
        </div>
        <div class="color__meta">
          <code>{{ parsed.rgb }}</code>
          <div class="color__meta-actions">
            <button class="color__btn color__btn--ghost" @click="copyHex">
              复制 HEX
            </button>
            <button class="color__btn color__btn--ghost" @click="copyRgb">
              复制 RGB
            </button>
          </div>
        </div>
      </div>
      <div v-else class="color__hint">
        <p>输入 HEX（如 <code>#FF8800</code>）或 RGB（如 <code>rgb(10, 20, 30)</code>）后点解析。</p>
        <p class="color__hint--sub">屏幕取色 (像素采样) 在 v0.2 提供。</p>
      </div>
    </div>
  </ToolShell>
</template>

<style scoped>
.color {
  display: flex;
  flex-direction: column;
  gap: 14px;
  flex: 1;
  min-height: 0;
}

.color__form {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.color__field {
  display: flex;
  align-items: center;
  gap: 8px;
}

.color__label {
  width: 48px;
  font-size: 11px;
  color: rgba(255, 255, 255, 0.5);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.color__input {
  flex: 1;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 6px;
  padding: 6px 10px;
  color: inherit;
  font: inherit;
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  font-size: 12px;
  outline: none;
}

.color__input:focus {
  border-color: rgba(102, 204, 255, 0.4);
}

.color__btn {
  background: rgba(102, 204, 255, 0.12);
  border: 1px solid rgba(102, 204, 255, 0.3);
  color: #6cf;
  padding: 5px 12px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
}

.color__btn:hover {
  background: rgba(102, 204, 255, 0.2);
}

.color__btn--ghost {
  background: transparent;
  border-color: rgba(255, 255, 255, 0.12);
  color: rgba(255, 255, 255, 0.7);
}

.color__btn--ghost:hover {
  background: rgba(255, 255, 255, 0.05);
  color: #f5f5f5;
}

.color__error {
  color: #ff8888;
  font-size: 12px;
}

.color__result {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.color__swatch {
  height: 100px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  font-size: 18px;
  letter-spacing: 1px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
}

.color__meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.7);
}

.color__meta code {
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  font-size: 12px;
  background: rgba(255, 255, 255, 0.06);
  padding: 4px 8px;
  border-radius: 4px;
}

.color__meta-actions {
  display: flex;
  gap: 6px;
}

.color__hint {
  color: rgba(255, 255, 255, 0.55);
  font-size: 12px;
  text-align: center;
  padding: 24px;
}

.color__hint--sub {
  color: rgba(255, 255, 255, 0.35);
  margin-top: 6px;
}

.color__hint code {
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  color: #6cf;
}

@media (prefers-color-scheme: light) {
  .color__btn--ghost {
    border-color: rgba(0, 0, 0, 0.15);
    color: rgba(0, 0, 0, 0.7);
  }
  .color__btn--ghost:hover {
    background: rgba(0, 0, 0, 0.04);
    color: #1a1a1a;
  }
  .color__input {
    background: rgba(0, 0, 0, 0.03);
    border-color: rgba(0, 0, 0, 0.1);
  }
  .color__meta code {
    background: rgba(0, 0, 0, 0.05);
  }
}
</style>