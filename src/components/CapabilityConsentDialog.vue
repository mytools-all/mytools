<script setup lang="ts">
import { computed } from "vue";
import { resolveCapabilityConsent, useCapabilityConsent } from "@/runtime/capabilityConsent";
const { consent } = useCapabilityConsent();
const consentTitle = computed(() => consent.value
  ? `允许「${consent.value.plugin}」使用 ${consent.value.capability}？` : "");
</script>

<template>
  <div v-if="consent" class="app-consent" data-testid="capability-consent">
    <div class="app-consent__panel">
      <p class="app-consent__title">{{ consentTitle }}</p>
      <p class="app-consent__text">{{ consent.message }}</p>
      <div class="app-consent__actions">
        <button
          type="button"
          class="app-consent__btn"
          data-testid="capability-consent-deny"
          @click="resolveCapabilityConsent(false)"
        >
          拒绝
        </button>
        <button
          type="button"
          class="app-consent__btn app-consent__btn--primary"
          data-testid="capability-consent-allow"
          @click="resolveCapabilityConsent(true)"
        >
          允许
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.app-consent {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.35);
  z-index: 40;
}

.app-consent__panel {
  width: min(360px, 80vw);
  padding: 16px 18px;
  border-radius: 12px;
  border: 1px solid var(--panel-border);
  background: var(--panel-bg);
  color: var(--panel-fg);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.28);
}

.app-consent__title {
  margin: 0 0 6px;
  font-size: 13px;
  font-weight: 600;
}

.app-consent__text {
  margin: 0 0 14px;
  font-size: 11px;
  color: var(--text-dim);
  word-break: break-all;
}

.app-consent__actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.app-consent__btn {
  padding: 5px 12px;
  border-radius: 6px;
  border: 1px solid var(--panel-control-border);
  background: transparent;
  color: var(--panel-fg);
  font-size: 12px;
  cursor: pointer;
}

.app-consent__btn:hover {
  background: var(--hover-bg);
}

.app-consent__btn--primary {
  border-color: var(--accent);
  color: var(--accent);
}
</style>
