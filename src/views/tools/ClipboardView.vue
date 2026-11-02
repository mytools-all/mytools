<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { useClipboardStore } from "@/stores/clipboard";
import ToolShell from "@/views/ToolShell.vue";

const store = useClipboardStore();

onMounted(async () => {
  await store.refresh();
  await store.startListening();
});

onUnmounted(() => {
  store.stopListening();
});

function timeAgo(iso: string): string {
  const t = new Date(iso).getTime();
  if (!Number.isFinite(t)) return "";
  const diff = Date.now() - t;
  const sec = Math.round(diff / 1000);
  if (sec < 60) return `${sec}s`;
  if (sec < 3600) return `${Math.round(sec / 60)}m`;
  if (sec < 86400) return `${Math.round(sec / 3600)}h`;
  return `${Math.round(sec / 86400)}d`;
}

function preview(content: string | undefined): string {
  if (!content) return "";
  return content.length > 200 ? content.slice(0, 200) + "…" : content;
}
</script>

<template>
  <ToolShell>
    <template #title>剪贴板历史</template>
    <div class="cb">
      <div class="cb__bar">
        <button class="cb__btn cb__btn--ghost" @click="store.refresh()">
          刷新
        </button>
        <button class="cb__btn cb__btn--ghost" @click="store.clearAll()">
          清空（保留置顶）
        </button>
        <span v-if="store.error" class="cb__error">{{ store.error }}</span>
      </div>

      <ul v-if="store.hasEntries" class="cb__list">
        <li
          v-for="(e, idx) in store.entries"
          :key="e.id"
          :class="['cb__item', idx === store.selectedIndex && 'is-selected', e.pinned && 'is-pinned']"
          @click="store.selectedIndex = idx"
        >
          <div class="cb__row">
            <span v-if="e.pinned" class="cb__pin">★</span>
            <span class="cb__time">{{ timeAgo(e.created_at) }}</span>
            <code class="cb__content">{{ preview(e.content) }}</code>
          </div>
          <div class="cb__actions">
            <button
              class="cb__btn cb__btn--ghost"
              @click.stop="store.copyToClipboard(e.content)"
            >
              复制
            </button>
            <button
              class="cb__btn cb__btn--ghost"
              @click.stop="store.pin(e.id, !e.pinned)"
            >
              {{ e.pinned ? "取消置顶" : "置顶" }}
            </button>
            <button class="cb__btn cb__btn--ghost" @click.stop="store.remove(e.id)">
              删除
            </button>
          </div>
        </li>
      </ul>
      <div v-else class="cb__empty">
        还没有剪贴板记录。复制一段文字试试。
      </div>
    </div>
  </ToolShell>
</template>

<style scoped>
.cb {
  display: flex;
  flex-direction: column;
  gap: 10px;
  flex: 1;
  min-height: 0;
}

.cb__bar {
  display: flex;
  gap: 6px;
  align-items: center;
}

.cb__btn {
  background: rgba(102, 204, 255, 0.12);
  border: 1px solid rgba(102, 204, 255, 0.3);
  color: #6cf;
  padding: 4px 10px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 11px;
}

.cb__btn:hover {
  background: rgba(102, 204, 255, 0.2);
}

.cb__btn--ghost {
  background: transparent;
  border-color: rgba(255, 255, 255, 0.12);
  color: rgba(255, 255, 255, 0.7);
}

.cb__btn--ghost:hover {
  background: rgba(255, 255, 255, 0.05);
  color: #f5f5f5;
}

.cb__error {
  color: #ff8888;
  font-size: 11px;
}

.cb__list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow-y: auto;
  flex: 1;
  min-height: 0;
}

.cb__item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 12px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.08s;
}

.cb__item:hover {
  background: rgba(255, 255, 255, 0.06);
}

.cb__item.is-selected {
  border-color: rgba(102, 204, 255, 0.4);
  background: rgba(102, 204, 255, 0.06);
}

.cb__item.is-pinned {
  border-left: 3px solid #fc6;
}

.cb__row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.cb__pin {
  color: #fc6;
  font-size: 12px;
}

.cb__time {
  color: rgba(255, 255, 255, 0.4);
  font-size: 11px;
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  flex-shrink: 0;
}

.cb__content {
  flex: 1;
  font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.9);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cb__actions {
  display: flex;
  gap: 4px;
}

.cb__empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.4);
  font-size: 12px;
}

@media (prefers-color-scheme: light) {
  .cb__btn--ghost {
    border-color: rgba(0, 0, 0, 0.15);
    color: rgba(0, 0, 0, 0.7);
  }
  .cb__btn--ghost:hover {
    background: rgba(0, 0, 0, 0.04);
    color: #1a1a1a;
  }
  .cb__item {
    background: rgba(0, 0, 0, 0.02);
    border-color: rgba(0, 0, 0, 0.08);
  }
  .cb__item.is-selected {
    background: rgba(102, 153, 204, 0.06);
  }
  .cb__content {
    color: rgba(0, 0, 0, 0.9);
  }
  .cb__time {
    color: rgba(0, 0, 0, 0.4);
  }
}
</style>