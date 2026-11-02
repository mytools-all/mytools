import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { tauriAvailable } from "@/lib/tauri";

export interface ClipboardEntry {
  id: number;
  content: string;
  pinned: boolean;
  created_at: string;
}

export const useClipboardStore = defineStore("clipboard", () => {
  const entries = ref<ClipboardEntry[]>([]);
  const selectedIndex = ref(0);
  const loading = ref(false);
  const error = ref<string | null>(null);
  let unlisten: UnlistenFn | null = null;

  async function refresh() {
    if (!tauriAvailable()) {
      entries.value = [];
      error.value = null;
      loading.value = false;
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      entries.value = await invoke<ClipboardEntry[]>("clipboard_list", {
        limit: 200,
      });
      selectedIndex.value = 0;
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function pin(id: number, pinned: boolean) {
    try {
      await invoke("clipboard_pin", { id, pinned });
      await refresh();
    } catch (e) {
      error.value = String(e);
    }
  }

  async function remove(id: number) {
    try {
      await invoke("clipboard_delete", { id });
      await refresh();
    } catch (e) {
      error.value = String(e);
    }
  }

  async function clearAll() {
    try {
      await invoke("clipboard_clear");
      await refresh();
    } catch (e) {
      error.value = String(e);
    }
  }

  async function copyToClipboard(text: string) {
    if (!tauriAvailable()) {
      try {
        await navigator.clipboard?.writeText(text);
      } catch {
        /* clipboard unavailable */
      }
      return;
    }
    try {
      await invoke("clipboard_write", { text });
    } catch (e) {
      error.value = String(e);
    }
  }

  async function startListening() {
    if (unlisten || !tauriAvailable()) return;
    try {
      unlisten = await listen<string>("clipboard:changed", () => {
        void refresh();
      });
    } catch (e) {
      error.value = String(e);
    }
  }

  function stopListening() {
    unlisten?.();
    unlisten = null;
  }

  const selected = computed(() => entries.value[selectedIndex.value] ?? null);
  const hasEntries = computed(() => entries.value.length > 0);

  function move(delta: number) {
    if (!hasEntries.value) return;
    const n = entries.value.length;
    selectedIndex.value = (selectedIndex.value + delta + n) % n;
  }

  return {
    entries,
    selectedIndex,
    loading,
    error,
    selected,
    hasEntries,
    refresh,
    pin,
    remove,
    clearAll,
    copyToClipboard,
    startListening,
    stopListening,
    move,
  };
});