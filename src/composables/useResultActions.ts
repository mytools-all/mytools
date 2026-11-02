import { useRouter } from "vue-router";
import { useSearchStore, type SearchResult } from "@/stores/search";
import { useUsageStore } from "@/stores/usage";

export { encodeRouteTarget } from "@/stores/search";

/**
 * Single dispatch point for "the user picked a search result".
 * Used by both the global header search box (keyboard Enter) and the
 * launcher result list (mouse click) so the two never drift apart.
 *
 * Every branch that navigates clears the search box — the query has
 * served its purpose once the user has left the launcher.
 */
export function useResultActions() {
  const router = useRouter();
  const search = useSearchStore();
  const usage = useUsageStore();

  async function run(item: SearchResult): Promise<void> {
    switch (item.kind) {
      case "calculator":
        navigator.clipboard?.writeText(item.title).catch(() => {});
        return;
      case "json-formatter":
        await router.push({ name: "json-formatter" });
        break;
      case "base64-url":
        await router.push({ name: "base64-url" });
        break;
      case "timestamp-uuid":
        await router.push({ name: "timestamp-uuid" });
        break;
      case "color-picker":
        await router.push({ name: "color-picker" });
        break;
      case "clipboard":
        await router.push({ name: "clipboard" });
        break;
      case "store":
        await router.push({ path: "/plugins", query: { section: "store" } });
        break;
      case "plugins":
        await router.push({ path: "/plugins", query: { section: "installed" } });
        break;
      case "plugin": {
        const [name, code] = item.id.split(":");
        if (!name || !code) return;
        usage.bump(name);
        await router.push({ name: "plugin", params: { id: name, code } });
        break;
      }
      case "route": {
        const [path, qs] = item.id.split("?");
        const query: Record<string, string> = {};
        if (qs) {
          for (const [k, v] of new URLSearchParams(qs).entries()) {
            query[k] = v;
          }
        }
        if (!path || !path.startsWith("/")) {
          await router.push({ name: "launcher" });
        } else {
          await router.push({ path, query });
        }
        break;
      }
      case "app": {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("app_launcher_open", { path: item.id });
          const { getCurrentWindow } = await import("@tauri-apps/api/window");
          await getCurrentWindow().hide();
          const { useUsageStore } = await import("@/stores/usage");
          const usage = useUsageStore();
          usage.bumpApp(item.id);
        } catch (e) {
          console.error("[MyTools] launch failed:", e);
        }
        break;
      }
      default:
        return;
    }
    search.reset();
  }

  /** Detach the currently-highlighted plugin result into its own window. */
  async function detachSelected(): Promise<void> {
    const item = search.results[search.selectedIndex];
    if (!item || item.kind !== "plugin") return;
    const [pluginId, feature] = item.id.split(":");
    if (!pluginId || !feature) return;
    const { safeInvoke } = await import("@/lib/tauri");
    await safeInvoke<string>("plugin_open_window", { pluginId, feature });
    search.reset();
  }

  return { run, detachSelected };
}