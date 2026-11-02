<script setup lang="ts">
interface Tab {
  id: string;
  label: string;
  count?: number;
  tone?: "ok" | "warn" | "err";
  title?: string;
  disabled?: boolean;
}

defineProps<{
  tabs: Tab[];
  modelValue: string;
}>();

defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>

<template>
  <div class="ttn">
    <button
      v-for="t in tabs"
      :key="t.id"
      type="button"
      class="ttn__tab"
      :class="[
        { 'ttn__tab--active': modelValue === t.id },
        t.tone ? `ttn__tab--${t.tone}` : '',
      ]"
      :disabled="t.disabled"
      :title="t.title"
      @click="$emit('update:modelValue', t.id)"
    >
      <span class="ttn__label">{{ t.label }}</span>
      <span v-if="t.count !== undefined" class="ttn__count">{{ t.count }}</span>
    </button>
  </div>
</template>

<style scoped>
.ttn {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  flex: none;
}

.ttn__tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  border-radius: 8px;
  border: 1px solid var(--panel-control-border);
  background: transparent;
  color: var(--text-dim);
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}

.ttn__tab:hover:not(:disabled) {
  background: var(--hover-bg);
  color: var(--panel-fg);
}

.ttn__tab:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.ttn__tab--active {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}

.ttn__count {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 8px;
  background: var(--hover-bg);
  color: var(--text-dim-strong);
}

.ttn__tab--active .ttn__count {
  background: transparent;
  color: var(--accent);
}

.ttn__tab--ok.ttn__tab--active {
  border-color: #98c379;
  color: #98c379;
}

.ttn__tab--err.ttn__tab--active {
  border-color: #e06c75;
  color: #e06c75;
}
</style>