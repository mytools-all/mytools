<script setup lang="ts">
import { ref } from "vue";
import ToolShell from "@/views/ToolShell.vue";

const input = ref("");
const output = ref("");
const error = ref<string | null>(null);
const mode = ref<"format" | "minify">("format");

function tryFormat() {
  error.value = null;
  const text = input.value.trim();
  if (!text) {
    output.value = "";
    return;
  }
  try {
    const parsed = JSON.parse(text);
    output.value =
      mode.value === "format"
        ? JSON.stringify(parsed, null, 2)
        : JSON.stringify(parsed);
  } catch (e) {
    output.value = "";
    error.value = `JSON 解析失败：${e instanceof Error ? e.message : String(e)}`;
  }
}

function clearAll() {
  input.value = "";
  output.value = "";
  error.value = null;
}

async function copyOutput() {
  if (!output.value) return;
  await navigator.clipboard?.writeText(output.value).catch(() => {});
}

async function pasteInput() {
  try {
    const text = await navigator.clipboard.readText();
    input.value = text;
    tryFormat();
  } catch {
    error.value = "无法读取剪贴板（需要用户授权）";
  }
}
</script>

<template>
  <ToolShell>
    <template #title>JSON 格式化</template>
    <div class="json">
      <div class="json__bar">
        <button class="json__btn json__btn--ghost" @click="pasteInput">
          粘贴
        </button>
        <select v-model="mode" class="json__select">
          <option value="format">美化 (2 空格)</option>
          <option value="minify">压缩</option>
        </select>
        <button class="json__btn" @click="tryFormat">格式化</button>
        <button class="json__btn json__btn--ghost" @click="copyOutput">
          复制结果
        </button>
        <button class="json__btn json__btn--ghost" @click="clearAll">
          清空
        </button>
      </div>
      <div v-if="error" class="json__error">{{ error }}</div>
      <div class="json__panes">
        <textarea
          v-model="input"
          class="json__textarea"
          placeholder='{"hello":"world"}'
          spellcheck="false"
          autocomplete="off"
        />
        <textarea
          v-model="output"
          class="json__textarea json__textarea--readonly"
          readonly
          placeholder="结果会显示在这里"
          spellcheck="false"
        />
      </div>
    </div>
  </ToolShell>
</template>

<style scoped>
.json {
  display: flex;
  flex-direction: column;
  gap: 10px;
  flex: 1;
  min-height: 0;
}

.json__bar {
  display: flex;
  gap: 6px;
  align-items: center;
}

.json__btn {
  background: rgba(102, 204, 255, 0.12);
  border: 1px solid rgba(102, 204, 255, 0.3);
  color: #6cf;
  padding: 5px 10px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
}

.json__btn:hover {
  background: rgba(102, 204, 255, 0.2);
}

.json__btn--ghost {
  background: transparent;
  border-color: rgba(255, 255, 255, 0.12);
  color: rgba(255, 255, 255, 0.7);
}

.json__btn--ghost:hover {
  background: rgba(255, 255, 255, 0.05);
  color: #f5f5f5;
}

.json__select {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: inherit;
  padding: 5px 8px;
  border-radius: 6px;
  font-size: 12px;
}

.json__error {
  color: #ff8888;
  font-size: 12px;
}

.json__panes {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  flex: 1;
  min-height: 0;
}

.json__textarea {
  width: 100%;
  height: 100%;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 10px 12px;
  color: inherit;
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  font-size: 12px;
  line-height: 1.5;
  outline: none;
  resize: none;
}

.json__textarea:focus {
  border-color: rgba(102, 204, 255, 0.4);
}

.json__textarea--readonly {
  background: rgba(102, 204, 255, 0.04);
  border-color: rgba(102, 204, 255, 0.15);
}

@media (prefers-color-scheme: light) {
  .json__btn--ghost {
    border-color: rgba(0, 0, 0, 0.15);
    color: rgba(0, 0, 0, 0.7);
  }
  .json__btn--ghost:hover {
    background: rgba(0, 0, 0, 0.04);
    color: #1a1a1a;
  }
  .json__textarea {
    background: rgba(0, 0, 0, 0.02);
    border-color: rgba(0, 0, 0, 0.1);
  }
  .json__textarea--readonly {
    background: rgba(102, 153, 204, 0.04);
  }
  .json__select {
    background: rgba(0, 0, 0, 0.03);
    border-color: rgba(0, 0, 0, 0.15);
  }
}
</style>