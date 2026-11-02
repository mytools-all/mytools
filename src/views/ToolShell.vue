<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";

const router = useRouter();
const title = ref("");

onMounted(() => {
  console.log("[MyTools] tool shell mounted");
});

function goBack() {
  router.push({ name: "launcher" });
}

defineExpose({ goBack });
</script>

<template>
  <div class="tool">
    <header class="tool__header" data-tauri-drag-region="deep">
      <button class="tool__back" @click="goBack" aria-label="返回">
        ←
      </button>
      <h2 class="tool__title">
        <slot name="title">{{ title }}</slot>
      </h2>
    </header>
    <main class="tool__body">
      <slot />
    </main>
  </div>
</template>

<style scoped>
.tool {
  width: 100%;
  height: 100%;
  background: var(--panel-bg-strong);
  color: var(--panel-fg);
  display: flex;
  flex-direction: column;
  font-family:
    -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC",
    "Microsoft YaHei", sans-serif;
  border-radius: 14px;
  overflow: hidden;
}

.tool__header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 18px;
  border-bottom: 1px solid var(--panel-border);
  flex: none;
}

.tool__back {
  background: transparent;
  border: 1px solid var(--panel-control-border);
  color: var(--text-dim);
  padding: 4px 10px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}

.tool__back:hover {
  background: var(--hover-bg);
  color: var(--panel-fg);
}

.tool__title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--panel-fg);
}

.tool__body {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 16px 18px;
}
</style>