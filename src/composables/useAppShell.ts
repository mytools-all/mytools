import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { listen } from "@tauri-apps/api/event";
import { usePluginsStore } from "@/stores/plugins";
import { useSettingsStore } from "@/stores/settings";
import { useAppIndexStore } from "@/stores/appIndex";
import { useWebviewHealth } from "@/composables/useWebviewHealth";
import { useWindowLifecycle } from "@/composables/useWindowLifecycle";
import { reportSystemError } from "@/composables/systemErrors";
import { safeInvoke, tauriAvailable } from "@/lib/tauri";

export function useAppShell() {
  const windowLabel = ref("");
  const isFloatballWindow = computed(() => windowLabel.value === "floatball");
  const isPluginWindow = computed(() => windowLabel.value.startsWith("plugin-"));
  const isPanelWindow = computed(() => windowLabel.value === "panel");
  const isCenterWindow = computed(() => windowLabel.value === "center");
  const plugins = usePluginsStore();
  const settings = useSettingsStore();
  const appIndex = useAppIndexStore();
  const router = useRouter();
  const route = useRoute();
  let disposed = false;
  let unlistenTray: (() => void) | undefined;

  useWebviewHealth();
  useWindowLifecycle({ onFocusGained: () => { void appIndex.refreshIfStale(); }, keepVisibleOnBlur: () => route.name === "plugin" || isPluginWindow.value || isCenterWindow.value });

  async function handleTrayAction(action: string) {
    if (disposed || windowLabel.value !== "main") return;
    try {
      if (action === "settings") {
        await safeInvoke("window_show");
        if (!disposed) await router.push({ path: "/plugins", query: { section: "settings" } });
      } else if (action === "show-launcher") {
        await safeInvoke("window_show");
        if (!disposed && route.name !== "launcher") await router.push({ name: "launcher" });
      } else if (action === "hide-launcher") {
        await safeInvoke("window_hide");
      }
    } catch (error) {
      reportSystemError("tray", error instanceof Error ? error.message : String(error));
    }
  }

  onMounted(async () => {
    if (tauriAvailable()) {
      try {
        const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
        windowLabel.value = getCurrentWebviewWindow().label;
      } catch (error) {
        reportSystemError("window", error instanceof Error ? error.message : String(error));
      }
    }
    if (disposed) return;
    if (!isFloatballWindow.value && !isPluginWindow.value && !isCenterWindow.value && !isPanelWindow.value) void appIndex.ensureLoaded();
    if (!isFloatballWindow.value) await plugins.startListening();
    if (disposed) { plugins.stopListening(); return; }
    const results = await Promise.allSettled([settings.applyTheme(), plugins.ensureLoaded()]);
    if (disposed) return;
    for (const result of results) {
      if (result.status === "rejected") reportSystemError("startup", String(result.reason));
    }
    if (tauriAvailable() && windowLabel.value === "main") {
      try {
        const unlisten = await listen<string>("tray:menu", (event) => {
          void handleTrayAction(event.payload);
        });
        if (disposed) unlisten();
        else unlistenTray = unlisten;
      } catch (error) {
        reportSystemError("tray", error instanceof Error ? error.message : String(error));
      }
    }
  });

  onBeforeUnmount(() => {
    disposed = true;
    unlistenTray?.();
    if (!isFloatballWindow.value) plugins.stopListening();
  });

  return { windowLabel, isFloatballWindow, isPluginWindow, isPanelWindow, isCenterWindow };
}
