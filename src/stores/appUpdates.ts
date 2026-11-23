import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { safeInvoke, tauriAvailable } from "@/lib/tauri";
import { APP_INFO } from "@/config/app";

export type UpdateMode = "disabled" | "notify" | "download";
export interface UpdateSnapshot {
  currentVersion: string;
  latestVersion: string | null;
  status: "unknown" | "checking" | "upToDate" | "available" | "localNewer" | "error";
  mode: UpdateMode;
  checkedAt: string | null;
  error: string | null;
  phase: "idle" | "downloading" | "ready" | "installing";
  downloadedBytes: number;
  totalBytes: number | null;
  canInstall: boolean;
  releaseUrl: string | null;
  revision: number;
}

export const useAppUpdatesStore = defineStore("appUpdates", () => {
  const state = ref<UpdateSnapshot>({ currentVersion: APP_INFO.version, latestVersion: null, status: "unknown", mode: "notify", checkedAt: null, error: null, phase: "idle", downloadedBytes: 0, totalBytes: null, canInstall: false, releaseUrl: null, revision: 0 });
  const error = ref<string | null>(null);
  const busy = ref(false);
  const savingMode = ref(false);
  const autostart = ref<boolean | null>(null);
  const autostartError = ref<string | null>(null);
  const savingAutostart = ref(false);
  const dismissedVersion = ref<string | null>(null);
  let initialization: Promise<void> | null = null;
  let stop: UnlistenFn | null = null;
  let generation = 0;
  const statusText = computed(() => {
    const value = state.value;
    if (value.phase === "ready") return `新版本 ${value.latestVersion} 已下载，等待安装`;
    if (value.phase === "installing") return "正在安装更新…";
    if (value.phase === "downloading") {
      const progress = value.totalBytes ? ` ${Math.min(100, Math.round(value.downloadedBytes / value.totalBytes * 100))}%` : "";
      return `正在下载更新${progress}`;
    }
    switch (value.status) {
      case "checking": return "正在检查更新…";
      case "error": return "检查失败，请重试";
      case "upToDate": return "已经是最新版了";
      case "available": return `当前最新版本 ${value.latestVersion}，可更新`;
      case "localNewer": return `正式最新版 ${value.latestVersion}，当前为较新版本`;
      default: return "尚未检查更新";
    }
  });
  const working = computed(() => busy.value || state.value.status === "checking" || ["downloading", "installing"].includes(state.value.phase));
  const showReminder = computed(() => state.value.mode !== "disabled" && state.value.status === "available" && dismissedVersion.value !== state.value.latestVersion);

  function accept(value: UpdateSnapshot | null) {
    if (value && value.revision >= state.value.revision) state.value = value;
  }

  async function refreshAutostart() {
    if (!tauriAvailable()) return;
    try {
      const value = await safeInvoke<boolean>("app_autostart_status");
      if (value !== null) autostart.value = value;
      autostartError.value = null;
    } catch (reason) { autostartError.value = `读取开机自启状态失败：${reason}`; }
  }

  function initialize() {
    if (initialization) return initialization;
    initialization = (async () => {
      if (!tauriAvailable()) return;
      const current = generation;
      try {
        const unlisten = await listen<UpdateSnapshot>("mytools:app-update", event => accept(event.payload));
        if (current !== generation) { unlisten(); return; }
        stop = unlisten;
        accept(await safeInvoke<UpdateSnapshot>("app_update_status"));
      } catch (reason) {
        stop?.(); stop = null;
        initialization = null;
        error.value = `读取更新状态失败：${reason}`;
      }
      await refreshAutostart();
    })();
    return initialization;
  }

  async function run(command: string, args?: Record<string, unknown>) {
    if (working.value || !tauriAvailable()) return;
    busy.value = true;
    error.value = null;
    try { accept(await safeInvoke<UpdateSnapshot>(command, args)); }
    catch (reason) { error.value = String(reason); }
    finally { busy.value = false; }
  }

  async function setMode(mode: UpdateMode) {
    if (savingMode.value || !tauriAvailable()) return;
    savingMode.value = true;
    error.value = null;
    try { accept(await safeInvoke<UpdateSnapshot>("app_update_set_mode", { mode })); }
    catch (reason) { error.value = `保存更新策略失败：${reason}`; }
    finally { savingMode.value = false; }
  }

  async function setAutostart(enabled: boolean) {
    if (savingAutostart.value || !tauriAvailable()) return;
    savingAutostart.value = true;
    autostartError.value = null;
    try {
      const value = await safeInvoke<boolean>("app_autostart_set", { enabled });
      if (value !== null) autostart.value = value;
    } catch (reason) { autostartError.value = `设置开机自启失败：${reason}`; }
    finally { savingAutostart.value = false; }
  }

  function dispose() { generation++; stop?.(); stop = null; initialization = null; }
  return { state, error, busy, savingMode, autostart, autostartError, savingAutostart, statusText, working, showReminder, initialize, refreshAutostart, setMode, setAutostart, dispose, check: () => run("app_update_check"), download: () => run("app_update_download"), install: () => run("app_update_install", { savedAndConfirmed: true }), dismiss: () => { dismissedVersion.value = state.value.latestVersion; } };
});
