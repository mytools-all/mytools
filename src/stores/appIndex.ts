import { defineStore } from "pinia";
import { ref } from "vue";
import { safeInvoke, tauriAvailable } from "@/lib/tauri";

export interface ListedApp {
  name: string;
  target: string;
  subtitle: string;
}

export interface ResolvedApp {
  name: string;
  subtitle: string;
  icon: string | null;
}

const REFRESH_THROTTLE_MS = 30_000;

function targetKey(target: string): string {
  return target.toLowerCase();
}

export const useAppIndexStore = defineStore("appIndex", () => {
  const entries = ref<ListedApp[]>([]);
  const byTarget = ref<Map<string, ListedApp>>(new Map());
  const icons = ref<Map<string, string | null>>(new Map());
  const loading = ref(false);
  const loaded = ref(false);

  let inflight: Promise<void> | null = null;
  let lastRefreshAt = 0;
  const pendingIcons = new Map<string, Promise<void>>();
  const iconQueue: Array<() => Promise<void>> = [];
  let activeIcons = 0;

  function drainIcons() {
    while (activeIcons < 3 && iconQueue.length > 0) {
      const task = iconQueue.shift()!;
      activeIcons++;
      void task().finally(() => { activeIcons--; drainIcons(); });
    }
  }

  function apply(list: ListedApp[]) {
    entries.value = list;
    const next = new Map<string, ListedApp>();
    for (const entry of list) next.set(targetKey(entry.target), entry);
    byTarget.value = next;
    loaded.value = true;
  }

  async function runLoad() {
    loading.value = true;
    try {
      const list = await safeInvoke<ListedApp[]>("app_index_list");
      if (Array.isArray(list)) {
        apply(list);
      } else {
        loaded.value = true;
      }
    } catch (err) {
      console.warn("[MyTools] app index load failed:", err);
      loaded.value = true;
    } finally {
      loading.value = false;
    }
  }

  async function ensureLoaded(force = false): Promise<void> {
    if (loaded.value && !force) return;
    if (inflight) return inflight;
    if (!tauriAvailable()) {
      loaded.value = true;
      return;
    }
    inflight = runLoad();
    try {
      await inflight;
    } finally {
      inflight = null;
    }
  }

  async function refreshIfStale(): Promise<void> {
    if (!tauriAvailable()) return;
    const now = Date.now();
    if (now - lastRefreshAt < REFRESH_THROTTLE_MS) return;
    lastRefreshAt = now;
    try {
      const refreshed = await safeInvoke<boolean>("app_index_refresh_if_stale");
      if (refreshed) await ensureLoaded(true);
    } catch (err) {
      console.warn("[MyTools] app index refresh failed:", err);
    }
  }

  function find(target: string): ListedApp | undefined {
    return byTarget.value.get(targetKey(target));
  }

  function iconFor(target: string): string | undefined {
    return icons.value.get(targetKey(target)) ?? undefined;
  }

  async function resolveIcon(target: string): Promise<string | null> {
    try {
      const resolved = await safeInvoke<ResolvedApp>("app_resolve", { target });
      if (resolved?.icon) return resolved.icon;
    } catch (err) {
      console.warn("[MyTools] app icon resolve failed:", err);
    }
    try {
      return (await safeInvoke<string | null>("app_icon", { path: target })) ?? null;
    } catch (err) {
      console.warn("[MyTools] app icon fallback failed:", err);
      return null;
    }
  }

  async function ensureIcon(target: string): Promise<void> {
    const key = targetKey(target);
    if (icons.value.has(key) || !tauriAvailable()) return;
    if (pendingIcons.has(key)) return pendingIcons.get(key);
    let finish!: () => void;
    const pending = new Promise<void>((resolve) => { finish = resolve; });
    pendingIcons.set(key, pending);
    iconQueue.push(async () => {
      try { icons.value.set(key, await resolveIcon(target)); }
      finally { pendingIcons.delete(key); finish(); }
    });
    drainIcons();
    return pending;
  }

  return {
    entries,
    loading,
    loaded,
    ensureLoaded,
    refreshIfStale,
    find,
    iconFor,
    ensureIcon,
  };
});
