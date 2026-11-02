<script setup lang="ts">
import { ref, computed } from "vue";
import ToolShell from "@/views/ToolShell.vue";

const expr = ref("");
const result = ref<string | null>(null);
const error = ref<string | null>(null);

const preview = computed(() => {
  const v = expr.value.trim();
  if (!v) return "输入表达式即可计算";
  if (!/^[\d+\-*/().%^\s,a-z]+$/i.test(v)) return "包含不支持的字符";
  return `= ${v}`;
});

function tryCalc() {
  error.value = null;
  try {
    const hasTauri = !!(window as unknown as { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
    if (hasTauri) {
      import("@tauri-apps/api/core")
        .then(({ invoke }) =>
          invoke("search", { query: `=${expr.value}` }).then(
            (rows: unknown) => {
              const list = rows as { title: string }[];
              if (list && list[0]) {
                result.value = list[0].title;
              } else {
                error.value = "表达式无效";
                result.value = null;
              }
            },
          ),
        )
        .catch((e) => {
          error.value = String(e);
          result.value = null;
        });
      return;
    }
    result.value = expr.value;
  } catch (e) {
    error.value = String(e);
    result.value = null;
  }
}

function clear() {
  expr.value = "";
  result.value = null;
  error.value = null;
}

async function copyResult() {
  if (!result.value) return;
  await navigator.clipboard?.writeText(result.value).catch(() => {});
}
</script>

<template>
  <ToolShell>
    <template #title>计算器</template>
    <div class="calc">
      <input
        v-model="expr"
        class="calc__input"
        type="text"
        placeholder="例如  (1+2)*sqrt(9)"
        spellcheck="false"
        autocomplete="off"
        @keydown.enter="tryCalc"
      />
      <div class="calc__row">
        <span class="calc__preview">{{ preview }}</span>
        <div class="calc__actions">
          <button class="calc__btn" @click="tryCalc">计算 (=)</button>
          <button class="calc__btn calc__btn--ghost" @click="clear">清空</button>
          <button
            v-if="result"
            class="calc__btn calc__btn--ghost"
            @click="copyResult"
          >
            复制
          </button>
        </div>
      </div>
      <div v-if="error" class="calc__error">{{ error }}</div>
      <div v-else-if="result" class="calc__result">
        <span class="calc__result-label">结果</span>
        <span class="calc__result-value">{{ result }}</span>
      </div>
    </div>
  </ToolShell>
</template>

<style scoped>
.calc {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-top: 8px;
}

.calc__input {
  width: 100%;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 10px 12px;
  color: inherit;
  font: inherit;
  font-size: 14px;
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  outline: none;
}

.calc__input:focus {
  border-color: rgba(102, 204, 255, 0.5);
}

.calc__row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.55);
}

.calc__actions {
  display: flex;
  gap: 6px;
}

.calc__btn {
  background: rgba(102, 204, 255, 0.12);
  border: 1px solid rgba(102, 204, 255, 0.3);
  color: #6cf;
  padding: 5px 10px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
}

.calc__btn:hover {
  background: rgba(102, 204, 255, 0.2);
}

.calc__btn--ghost {
  background: transparent;
  border-color: rgba(255, 255, 255, 0.12);
  color: rgba(255, 255, 255, 0.7);
}

.calc__btn--ghost:hover {
  background: rgba(255, 255, 255, 0.05);
  color: #f5f5f5;
}

.calc__error {
  color: #ff8888;
  font-size: 12px;
}

.calc__result {
  display: flex;
  align-items: baseline;
  gap: 12px;
  background: rgba(102, 204, 255, 0.06);
  padding: 12px 14px;
  border-radius: 8px;
  border-left: 3px solid #6cf;
}

.calc__result-label {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.5);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.calc__result-value {
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  font-size: 16px;
  color: #6cf;
}

@media (prefers-color-scheme: light) {
  .calc__input {
    background: rgba(0, 0, 0, 0.03);
    border-color: rgba(0, 0, 0, 0.1);
  }
  .calc__btn--ghost {
    border-color: rgba(0, 0, 0, 0.15);
    color: rgba(0, 0, 0, 0.7);
  }
  .calc__btn--ghost:hover {
    background: rgba(0, 0, 0, 0.04);
    color: #1a1a1a;
  }
  .calc__result {
    background: rgba(102, 153, 204, 0.06);
  }
}
</style>