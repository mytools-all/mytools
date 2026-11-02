<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from "vue";
import { useAppIndexStore } from "@/stores/appIndex";
import type { SearchResult } from "@/stores/search";
import { fallbackIconDataUri } from "@/lib/iconFallback";
import { isAppPinKey, toAppPinKey } from "@/stores/usage";

const props = defineProps<{
  results: SearchResult[];
  selectedIndex: number;
  pinnedIds?: Set<string>;
}>();

const emit = defineEmits<{
  select: [item: SearchResult];
  pin: [item: SearchResult];
}>();

const appIndex = useAppIndexStore();
const listEl = ref<HTMLElement | null>(null);
let observer: IntersectionObserver | null = null;
const kindLabels: Record<string, string> = { app: "应用", plugin: "插件", calculator: "计算", route: "导航", store: "工具", plugins: "管理" };

function observeIcons() {
  observer?.disconnect();
  if (!listEl.value) return;
  if (typeof IntersectionObserver !== "function") {
    for (const item of props.results.filter((item) => item.kind === "app").slice(0, 8)) void appIndex.ensureIcon(item.id);
    return;
  }
  observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      const target = (entry.target as HTMLElement).dataset.iconTarget;
      if (target) void appIndex.ensureIcon(target);
      observer?.unobserve(entry.target);
    }
  }, { root: listEl.value });
  listEl.value.querySelectorAll("[data-icon-target]").forEach((element) => observer?.observe(element));
}

onMounted(observeIcons);
watch(() => props.results, observeIcons, { flush: "post" });
watch(() => props.selectedIndex, () => {
  listEl.value?.querySelector<HTMLElement>(".is-selected")?.scrollIntoView?.({ block: "nearest" });
}, { flush: "post" });
onBeforeUnmount(() => {
  observer?.disconnect();
  for (const timer of pressTimers.values()) clearTimeout(timer);
});

const LONG_PRESS_MS = 500;
const pressing = ref<Set<string>>(new Set());
const pressTimers: Map<string, ReturnType<typeof setTimeout>> = new Map();

const pinned = computed(() => props.pinnedIds ?? new Set<string>());

function pluginName(item: SearchResult): string {
  return item.id.split(":")[0];
}

function pinKey(item: SearchResult): string {
  if (item.kind === "app") {
    return toAppPinKey(item.id);
  }
  if (item.kind === "plugin") {
    return pluginName(item);
  }
  return item.id;
}

function isPinned(item: SearchResult): boolean {
  const key = pinKey(item);
  if (isAppPinKey(key)) {
    return pinned.value.has(key);
  }
  if (item.kind === "plugin") {
    return pinned.value.has(pluginName(item));
  }
  return pinned.value.has(key);
}

function activate(item: SearchResult) {
  emit("select", item);
}

function iconFor(item: SearchResult): string {
  return item.icon || (item.kind === "app" ? appIndex.iconFor(item.id) : undefined) || fallbackIconDataUri(item.title);
}

function clearPressTimer(id: string) {
  const t = pressTimers.get(id);
  if (t) {
    clearTimeout(t);
    pressTimers.delete(id);
  }
  if (pressing.value.has(id)) {
    const next = new Set(pressing.value);
    next.delete(id);
    pressing.value = next;
  }
}

function onPressStart(item: SearchResult, e: PointerEvent) {
  if (e.button !== undefined && e.button !== 0) return;
  const id = item.id;
  if (pressing.value.has(id)) return;
  const next = new Set(pressing.value);
  next.add(id);
  pressing.value = next;
  pressTimers.set(
    id,
    setTimeout(() => {
      pressTimers.delete(id);
      const after = new Set(pressing.value);
      after.delete(id);
      pressing.value = after;
      emit("pin", item);
    }, LONG_PRESS_MS),
  );
}

function onPressMove(item: SearchResult) {
  clearPressTimer(item.id);
}

function onPressEnd(item: SearchResult) {
  clearPressTimer(item.id);
}

function onContextMenu(item: SearchResult) {
  emit("pin", item);
}
</script>

<template>
  <ul ref="listEl" class="result-list" role="listbox" aria-label="搜索结果">
    <li
      v-for="(item, idx) in results"
      :key="item.id"
      v-memo="[item, idx === selectedIndex, isPinned(item), pressing.has(item.id), appIndex.iconFor(item.id)]"
      :data-icon-target="item.kind === 'app' && !item.icon ? item.id : undefined"
      :class="['result-list__item', idx === selectedIndex && 'is-selected']"
      role="option"
      :aria-selected="idx === selectedIndex"
      @click="activate(item)"
    >
      <span class="result-list__icon">
        <img :src="iconFor(item)" :alt="item.title" class="result-list__icon-img" />
      </span>
      <span class="result-list__kind">{{ kindLabels[item.kind] ?? "工具" }}</span>
      <span class="result-list__title">{{ item.title }}</span>
      <span class="result-list__trail">
        <span class="result-list__subtitle">{{ item.subtitle }}</span>
        <button
          v-if="item.kind === 'plugin' || item.kind === 'app'"
          type="button"
          class="result-list__pin"
          :class="{
            'result-list__pin--on': isPinned(item),
            'is-pressing': pressing.has(item.id),
          }"
          :title="isPinned(item) ? '取消固定' : '固定（左键 / 右键 / 长按）'"
          :aria-label="isPinned(item) ? '取消固定' : '固定'"
          @click.stop="emit('pin', item)"
          @contextmenu.stop.prevent="onContextMenu(item)"
          @pointerdown.stop="onPressStart(item, $event)"
          @pointermove.stop="onPressMove(item)"
          @pointerup.stop="onPressEnd(item)"
          @pointerleave.stop="onPressEnd(item)"
          @pointercancel.stop="onPressEnd(item)"
        >
          {{ isPinned(item) ? "★" : "☆" }}
        </button>
      </span>
    </li>
  </ul>
</template>

<style scoped>
.result-list {
  list-style: none;
  margin: 0;
  padding: 4px 0;
  flex: 1;
  min-height: 0;
  scrollbar-gutter: stable;
  overflow-y: auto;
  position: relative;
}

.result-list__item {
  display: grid;
  grid-template-columns: 32px 42px minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  margin: 3px 0;
  border-radius: 10px;
  min-height: 46px;
  cursor: pointer;
  color: var(--text-dim);
  font-size: 13px;
  border-left: 2px solid transparent;
  transition: background 0.08s ease;
}

.result-list__icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  flex: none;
}

.result-list__icon-img {
  width: 24px;
  height: 24px;
  object-fit: contain;
  border-radius: 4px;
}

.result-list__icon-placeholder {
  width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: var(--hover-bg);
  color: var(--text-dim-strong);
  font-size: 12px;
  font-weight: 600;
}

.result-list__item:hover {
  background: var(--hover-bg);
}

.result-list__item.is-selected {
  background: var(--accent-soft);
  border-left-color: var(--accent);
}

.result-list__kind {
  font-size: 10px;
  border-radius: 5px;
  padding: 2px 5px;
  background: var(--chip-bg);
  text-align: center;
  color: var(--text-dim-strong);
  text-transform: uppercase;
  letter-spacing: 0.4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.result-list__title {
  color: var(--panel-fg);
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-list__trail {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  justify-content: flex-end;
}

.result-list__pin {
  border: none;
  background: transparent;
  color: var(--text-dim-strong);
  font-size: 13px;
  cursor: pointer;
  line-height: 1;
  padding: 2px 4px;
  border-radius: 4px;
}

.result-list__pin:hover {
  color: #e5c07b;
}

.result-list__pin--on {
  color: #e5c07b;
}

.result-list__pin.is-pressing {
  transform: scale(1.25);
  color: var(--accent);
  transition: transform 0.08s ease;
}

.result-list__subtitle {
  color: var(--text-dim-strong);
  font-size: 11px;
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
@media (max-width: 600px) {
  .result-list__subtitle { display: none; }
}
</style>