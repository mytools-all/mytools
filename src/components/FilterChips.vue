<script setup lang="ts">
export interface ChipOption {
  id: string;
  label: string;
  count?: number;
  tone?: "ok" | "warn" | "err" | "placeholder";
  title?: string;
  disabled?: boolean;
}

defineProps<{
  label: string;
  options: ChipOption[];
  modelValue: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

function chipClass(opt: ChipOption, active: boolean): string[] {
  const classes = ["fcs__chip"];
  if (opt.tone) classes.push(`fcs__chip--${opt.tone}`);
  if (active) classes.push("fcs__chip--active");
  if (opt.disabled) classes.push("fcs__chip--disabled");
  return classes;
}

function select(opt: ChipOption) {
  if (opt.disabled) return;
  emit("update:modelValue", opt.id);
}
</script>

<template>
  <div class="fcs">
    <span class="fcs__label">{{ label }}</span>
    <button
      v-for="opt in options"
      :key="opt.id"
      type="button"
      :class="chipClass(opt, modelValue === opt.id)"
      :disabled="opt.disabled"
      :title="opt.title"
      @click="select(opt)"
    >
      <span class="fcs__chip-label">{{ opt.label }}</span>
      <span v-if="opt.count !== undefined" class="fcs__chip-count">
        {{ opt.count }}
      </span>
    </button>
    <slot />
  </div>
</template>

<style scoped>
.fcs {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.fcs__label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-dim-strong);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  margin-right: 2px;
}

.fcs__chip {
  padding: 4px 10px;
  border-radius: 14px;
  border: 1px solid var(--panel-control-border);
  background: transparent;
  color: var(--panel-fg);
  font-size: 12px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  transition: background 0.08s ease, border-color 0.08s ease;
}

.fcs__chip:hover:not(:disabled) {
  background: var(--hover-bg);
}

.fcs__chip:disabled {
  cursor: default;
}

.fcs__chip--active {
  border-color: var(--accent);
  background: var(--chip-bg, rgba(102, 204, 255, 0.12));
  color: var(--accent);
}

.fcs__chip--ok.fcs__chip--active {
  border-color: #4ade80;
  background: rgba(74, 222, 128, 0.12);
  color: #4ade80;
}

.fcs__chip--warn.fcs__chip--active {
  border-color: #e5c07b;
  background: rgba(229, 192, 123, 0.12);
  color: #e5c07b;
}

.fcs__chip--err.fcs__chip--active {
  border-color: #e06c75;
  background: rgba(224, 108, 117, 0.12);
  color: #e06c75;
}

.fcs__chip--placeholder {
  opacity: 0.5;
  cursor: not-allowed;
}

.fcs__chip--disabled {
  opacity: 0.5;
}

.fcs__chip-count {
  font-size: 10px;
  background: var(--hover-bg);
  border-radius: 10px;
  padding: 1px 6px;
  min-width: 18px;
  text-align: center;
}

.fcs__chip--active .fcs__chip-count {
  background: rgba(255, 255, 255, 0.12);
}
</style>