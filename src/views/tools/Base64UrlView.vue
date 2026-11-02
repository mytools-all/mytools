<script setup lang="ts">
import { ref } from "vue";
import ToolShell from "@/views/ToolShell.vue";

type Mode = "encode-base64" | "decode-base64" | "encode-url" | "decode-url";
const input = ref("");
const output = ref("");
const error = ref<string | null>(null);
const mode = ref<Mode>("encode-base64");

function utf8ToBytes(str: string): Uint8Array {
  return new TextEncoder().encode(str);
}

function bytesToUtf8(bytes: Uint8Array): string {
  return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
}

function tryRun() {
  error.value = null;
  const text = input.value;
  if (!text) {
    output.value = "";
    return;
  }
  try {
    switch (mode.value) {
      case "encode-base64": {
        const bytes = utf8ToBytes(text);
        let bin = "";
        for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
        output.value = btoa(bin);
        break;
      }
      case "decode-base64": {
        const bin = atob(text.replace(/\s+/g, ""));
        const bytes = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
        output.value = bytesToUtf8(bytes);
        break;
      }
      case "encode-url": {
        output.value = encodeURIComponent(text);
        break;
      }
      case "decode-url": {
        output.value = decodeURIComponent(text);
        break;
      }
    }
  } catch (e) {
    output.value = "";
    error.value = `失败：${e instanceof Error ? e.message : String(e)}`;
  }
}

async function copy() {
  if (!output.value) return;
  await navigator.clipboard?.writeText(output.value).catch(() => {});
}

function swap() {
  input.value = output.value;
  output.value = "";
}
</script>

<template>
  <ToolShell>
    <template #title>Base64 / URL 编解码</template>
    <div class="b64">
      <div class="b64__bar">
        <select v-model="mode" class="b64__select" @change="tryRun">
          <option value="encode-base64">Base64 编码</option>
          <option value="decode-base64">Base64 解码</option>
          <option value="encode-url">URL 编码</option>
          <option value="decode-url">URL 解码</option>
        </select>
        <button class="b64__btn" @click="tryRun">执行</button>
        <button class="b64__btn b64__btn--ghost" @click="swap">↑↓ 交换</button>
        <button class="b64__btn b64__btn--ghost" @click="copy">复制结果</button>
      </div>
      <div v-if="error" class="b64__error">{{ error }}</div>
      <div class="b64__panes">
        <textarea
          v-model="input"
          class="b64__textarea"
          placeholder="输入"
          spellcheck="false"
          @keyup="tryRun"
        />
        <textarea
          v-model="output"
          class="b64__textarea b64__textarea--readonly"
          readonly
          placeholder="结果"
          spellcheck="false"
        />
      </div>
    </div>
  </ToolShell>
</template>

<style scoped>
.b64 {
  display: flex;
  flex-direction: column;
  gap: 10px;
  flex: 1;
  min-height: 0;
}

.b64__bar {
  display: flex;
  gap: 6px;
}

.b64__select {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: inherit;
  padding: 5px 8px;
  border-radius: 6px;
  font-size: 12px;
}

.b64__btn {
  background: rgba(102, 204, 255, 0.12);
  border: 1px solid rgba(102, 204, 255, 0.3);
  color: #6cf;
  padding: 5px 10px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
}

.b64__btn:hover {
  background: rgba(102, 204, 255, 0.2);
}

.b64__btn--ghost {
  background: transparent;
  border-color: rgba(255, 255, 255, 0.12);
  color: rgba(255, 255, 255, 0.7);
}

.b64__btn--ghost:hover {
  background: rgba(255, 255, 255, 0.05);
  color: #f5f5f5;
}

.b64__error {
  color: #ff8888;
  font-size: 12px;
}

.b64__panes {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  flex: 1;
  min-height: 0;
}

.b64__textarea {
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

.b64__textarea:focus {
  border-color: rgba(102, 204, 255, 0.4);
}

.b64__textarea--readonly {
  background: rgba(102, 204, 255, 0.04);
  border-color: rgba(102, 204, 255, 0.15);
}

@media (prefers-color-scheme: light) {
  .b64__btn--ghost {
    border-color: rgba(0, 0, 0, 0.15);
    color: rgba(0, 0, 0, 0.7);
  }
  .b64__btn--ghost:hover {
    background: rgba(0, 0, 0, 0.04);
    color: #1a1a1a;
  }
  .b64__textarea {
    background: rgba(0, 0, 0, 0.02);
    border-color: rgba(0, 0, 0, 0.1);
  }
  .b64__textarea--readonly {
    background: rgba(102, 153, 204, 0.04);
  }
  .b64__select {
    background: rgba(0, 0, 0, 0.03);
    border-color: rgba(0, 0, 0, 0.15);
  }
}
</style>