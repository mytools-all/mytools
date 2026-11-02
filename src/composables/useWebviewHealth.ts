import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { safeInvoke } from "@/lib/tauri";
import { useRoute } from "vue-router";
import { reportSystemError } from "@/composables/systemErrors";
import { usePluginErrors } from "@/composables/pluginBridge";

export interface WebviewHealthSnapshot {
  my_pid: number;
  live_pids: number[];
  crash_count: number;
  last_crash_at: number | null;
}

const POLL_INTERVAL_MS = 5000;
const SYSTEM_KEY = "system:webview";

export function useWebviewHealth() {
  const health = ref<WebviewHealthSnapshot | null>(null);
  const errors = usePluginErrors();
  const route = useRoute();
  let timer: number | null = null;
  let lastSeenCrash = 0;
  let stopped = false;

  async function sample() {
    if (stopped) return;
    const raw = await safeInvoke<{
      my_pid: number;
      live_pids: number[];
      crash_count: number;
      last_crash_at: number | null;
    }>("webview_health");
    if (!raw) return;
    health.value = raw;
    await maybeReportCrash(raw);
  }

  async function maybeReportCrash(snapshot: WebviewHealthSnapshot) {
    if (snapshot.crash_count > lastSeenCrash) {
      const delta = snapshot.crash_count - lastSeenCrash;
      lastSeenCrash = snapshot.crash_count;
      const when = snapshot.last_crash_at
        ? new Date(snapshot.last_crash_at).toISOString()
        : "unknown";
      reportSystemError(
        "webview",
        `WebView 子进程异常退出 ${delta} 次`,
        `last_crash_at: ${when}\nlive_pids: ${snapshot.live_pids.join(", ") || "(none)"}`,
      );
    }
  }

  function start() {
    stopped = false;
    void sample();
    timer = window.setInterval(sample, POLL_INTERVAL_MS);
  }

  function stop() {
    stopped = true;
    if (timer != null) {
      window.clearInterval(timer);
      timer = null;
    }
    errors.clear(SYSTEM_KEY);
  }

  onMounted(() => {
    start();
  });

  onBeforeUnmount(() => {
    stop();
  });

  watch(
    () => route.name,
    () => {
      lastSeenCrash = 0;
      errors.clear(SYSTEM_KEY);
      void sample();
    },
  );

  return { health, start, stop, sample };
}