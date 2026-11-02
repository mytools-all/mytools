import { defineStore } from "pinia";
import { ref, computed, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { tauriAvailable } from "@/lib/tauri";

export interface SearchResult {
  kind: string;
  id: string;
  title: string;
  subtitle: string;
  score: number;
  icon?: string;
}

export type RouteTarget = {
  path: string;
  query?: Record<string, string>;
};

/**
 * Exact-match keyword → vue-router target.
 * User types "插件商店" + Enter → router.push below.
 */
export const ROUTE_COMMANDS: Readonly<Record<string, RouteTarget>> = {
  插件商店: { path: "/plugins", query: { section: "store" } },
  商店: { path: "/plugins", query: { section: "store" } },
  工具: { path: "/plugins", query: { section: "store" } },
  插件中心: { path: "/plugins", query: { section: "installed" } },
  插件管理: { path: "/plugins", query: { section: "installed" } },
  设置: { path: "/plugins", query: { section: "settings" } },
  偏好设置: { path: "/plugins", query: { section: "settings" } },
  settings: { path: "/plugins", query: { section: "settings" } },
};

const SETTINGS_ICON = `data:image/svg+xml,${encodeURIComponent(
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40"><rect width="40" height="40" rx="8" fill="#64748b"/><path fill="#fff" d="M17.2 8h5.6l.9 4.1c1.3.5 2.4 1.3 3.4 2.2l4-1.3 2.8 4.8-3.2 2.7c.1.7.2 1.4.2 2.1s-.1 1.5-.2 2.1l3.2 2.7-2.8 4.8-4-1.3c-1 .9-2.1 1.7-3.4 2.2l-.9 4.1h-5.6l-.9-4.1c-1.3-.5-2.4-1.3-3.4-2.2l-4 1.3-2.8-4.8 3.2-2.7c-.1-.7-.2-1.4-.2-2.1s.1-1.5.2-2.1l-3.2-2.7 2.8-4.8 4 1.3c1-.9 2.1-1.7 3.4-2.2L17.2 8zm2.8 11a3 3 0 1 0 0 6 3 3 0 0 0 0-6z"/></svg>',
)}`;

const PLUGINS_ICON = `data:image/svg+xml,${encodeURIComponent(
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40"><rect width="40" height="40" rx="8" fill="#4f46e5"/><rect x="11" y="11" width="8" height="8" rx="2" fill="#fff"/><rect x="21" y="11" width="8" height="8" rx="2" fill="#fff" opacity=".75"/><rect x="11" y="21" width="8" height="8" rx="2" fill="#fff" opacity=".75"/><rect x="21" y="21" width="8" height="8" rx="2" fill="#fff"/></svg>',
)}`;

const STORE_ICON = `data:image/svg+xml,${encodeURIComponent(
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40"><rect width="40" height="40" rx="8" fill="#0f766e"/><path d="M12 15h16l-1.6 12.2a2 2 0 0 1-2 1.8h-8.8a2 2 0 0 1-2-1.8L12 15z" fill="none" stroke="#fff" stroke-width="2"/><path d="M15.5 15a4.5 4.5 0 0 1 9 0" fill="none" stroke="#fff" stroke-width="2"/></svg>',
)}`;

export function matchRouteCommand(q: string): RouteTarget | null {
  const trimmed = q.trim();
  if (!trimmed) return null;
  const exact = ROUTE_COMMANDS[trimmed];
  if (exact) return exact;
  let best: { target: RouteTarget; len: number } | null = null;
  for (const [keyword, target] of Object.entries(ROUTE_COMMANDS)) {
    const hit = trimmed.includes(keyword) || (keyword.includes(trimmed) && trimmed.length >= 2);
    if (hit && (!best || keyword.length > best.len)) {
      best = { target, len: keyword.length };
    }
  }
  return best?.target ?? null;
}

/**
 * Encode a route-command target into a `SearchResult.id`.
 * Format: `<path>?k=v&k=v` so consumers can parse it with
 * `URLSearchParams` without any JSON payload.
 */
export function encodeRouteTarget(path: string, query?: Record<string, string>): string {
  const params = new URLSearchParams();
  for (const [k, v] of Object.entries(query ?? {})) {
    params.append(k, v);
  }
  const qs = params.toString();
  return qs ? `${path}?${qs}` : path;
}

/**
 * Stable per-target icon so every alias of the same route (设置 /
 * 偏好设置 / settings …) renders one identical glyph — the fallback
 * icon derives from the typed title, which differed per alias.
 */
function routeIcon(target: RouteTarget): string | undefined {
  const key = encodeRouteTarget(target.path, target.query);
  if (key === "/plugins?section=settings") return SETTINGS_ICON;
  if (key === "/plugins?section=store") return STORE_ICON;
  if (key === "/plugins?section=installed") return PLUGINS_ICON;
  return undefined;
}

/**
 * Turn an exact route-command match into a visible launcher result so the
 * user sees the row (not just a silent Enter shortcut). The backend search
 * knows nothing about these keywords, so without this the list stays empty.
 */
export function routeCommandResults(q: string): SearchResult[] {
  const target = matchRouteCommand(q);
  if (!target) return [];
  return [
    {
      kind: "route",
      id: encodeRouteTarget(target.path, target.query),
      title: q.trim(),
      subtitle: "路由命令",
      score: Number.MAX_SAFE_INTEGER,
      icon: routeIcon(target),
    },
  ];
}

export const useSearchStore = defineStore("search", () => {
  const query = ref("");
  const results = ref<SearchResult[]>([]);
  const selectedIndex = ref(0);
  const loading = ref(false);
  const error = ref<string | null>(null);

  let token = 0;

  async function refresh() {
    const q = query.value;
    const my = ++token;
    if (q.trim() === "") {
      results.value = [];
      selectedIndex.value = 0;
      error.value = null;
      return;
    }
    const routeHits = routeCommandResults(q);
    loading.value = true;
    error.value = null;
    if (!tauriAvailable()) {
      results.value = routeHits;
      selectedIndex.value = 0;
      loading.value = false;
      return;
    }
    try {
      const r = await invoke<SearchResult[]>("search", { query: q });
      if (my !== token) return;
      results.value = [...routeHits, ...r];
      selectedIndex.value = 0;
    } catch (e) {
      if (my !== token) return;
      error.value = String(e);
      results.value = routeHits;
    } finally {
      if (my === token) loading.value = false;
    }
  }

  let timer: ReturnType<typeof setTimeout> | null = null;
  function cancelScheduledRefresh() {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
  }
  watch(query, () => {
    cancelScheduledRefresh();
    timer = setTimeout(() => {
      timer = null;
      void refresh();
    }, 50);
  });

  async function refreshNow() {
    await nextTick();
    cancelScheduledRefresh();
    await refresh();
  }

  const hasResults = computed(() => results.value.length > 0);
  const selected = computed(() => results.value[selectedIndex.value] ?? null);

  function move(delta: number) {
    if (!hasResults.value) return;
    const n = results.value.length;
    selectedIndex.value = (selectedIndex.value + delta + n) % n;
  }

  function reset() {
    query.value = "";
    results.value = [];
    selectedIndex.value = 0;
  }

  return {
    query,
    results,
    selectedIndex,
    loading,
    error,
    hasResults,
    selected,
    move,
    reset,
    refresh,
    refreshNow,
  };
});