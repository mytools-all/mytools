import { onMounted, onUnmounted, getCurrentInstance } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { setupDragGuard, isDragGuardActive } from "@/lib/dragGuard";
import { safeInvoke } from "@/lib/tauri";

type WindowApi = {
  onFocusChanged: (
    cb: (event: { payload: boolean }) => void,
  ) => Promise<() => void>;
  onMoved: (cb: (event: { payload: unknown }) => void) => Promise<() => void>;
  onResized: (cb: (event: { payload: unknown }) => void) => Promise<() => void>;
  hide: () => Promise<void>;
  show: () => Promise<void>;
  setFocus: () => Promise<void>;
  outerPosition: () => Promise<{ x: number; y: number }>;
  outerSize: () => Promise<{ width: number; height: number }>;
};

const HIDE_DELAY_MS = 300;
const SAVE_BOUNDS_DEBOUNCE_MS = 400;
let saveBoundsTimer: ReturnType<typeof setTimeout> | null = null;

function cancelSaveBounds() {
  if (saveBoundsTimer !== null) {
    clearTimeout(saveBoundsTimer);
    saveBoundsTimer = null;
  }
}

function scheduleSaveBounds(appWindow: WindowApi) {
  cancelSaveBounds();
  saveBoundsTimer = setTimeout(async () => {
    saveBoundsTimer = null;
    try {
      const [pos, size] = await Promise.all([
        appWindow.outerPosition(),
        appWindow.outerSize(),
      ]);
      await safeInvoke("window_save_bounds", {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
      });
    } catch (err) {
      console.warn("[MyTools] save window bounds failed:", err);
    }
  }, SAVE_BOUNDS_DEBOUNCE_MS);
}

async function getWindow(): Promise<WindowApi> {
  const w = (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  if (!w) {
    return makeNoopWindow();
  }
  try {
    return getCurrentWindow() as unknown as WindowApi;
  } catch {
    return makeNoopWindow();
  }
}
function makeNoopWindow(): WindowApi {
  return {
    onFocusChanged: async () => () => {},
    onMoved: async () => () => {},
    onResized: async () => () => {},
    hide: async () => {},
    show: async () => {},
    setFocus: async () => {},
    outerPosition: async () => ({ x: 0, y: 0 }),
    outerSize: async () => ({ width: 0, height: 0 }),
  };
}

function currentWindowLabel(): string | undefined {
  const internals = (
    window as unknown as {
      __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } };
    }
  ).__TAURI_INTERNALS__;
  return internals?.metadata?.currentWindow?.label;
}

function isPluginCenterRoute(): boolean {
  return typeof window !== "undefined" && window.location.hash.startsWith("#/plugins");
}

export interface WindowLifecycleOptions {
  /** Fired when the main window regains focus (not for plugin/floatball). */
  onFocusGained?: () => void;
  keepVisibleOnBlur?: () => boolean;
}

export function useWindowLifecycle(options?: WindowLifecycleOptions) {
  let unlistenBlur: (() => void) | null = null;
  let unlistenMoved: (() => void) | null = null;
  let unlistenResized: (() => void) | null = null;
  let tearDownDragGuard: (() => void) | null = null;
  let hideTimer: ReturnType<typeof setTimeout> | null = null;

  function cancelHide() {
    if (hideTimer !== null) {
      clearTimeout(hideTimer);
      hideTimer = null;
    }
  }

  onMounted(() => {
    void getWindow().then((appWindow) => {
      if (currentWindowLabel() === "floatball") {
        return;
      }
      const isMain = currentWindowLabel() === "main";
      tearDownDragGuard = setupDragGuard();
      appWindow.onFocusChanged(({ payload: focused }) => {
        if (focused) {
          cancelHide();
          if (isMain) options?.onFocusGained?.();
          return;
        }
        if (isDragGuardActive() || options?.keepVisibleOnBlur?.()) {
          cancelHide();
          return;
        }
        cancelHide();
        hideTimer = setTimeout(() => {
          hideTimer = null;
          appWindow.hide().catch((err) => {
            console.warn("[MyTools] hide on blur failed:", err);
          });
        }, HIDE_DELAY_MS);
      }).then((unlisten) => {
        unlistenBlur = unlisten;
      });

      appWindow.onMoved(() => {
        cancelHide();
        if (isMain && !isPluginCenterRoute()) scheduleSaveBounds(appWindow);
      }).then((unlisten) => {
        unlistenMoved = unlisten;
      });

      appWindow.onResized(() => {
        cancelHide();
        if (isMain && !isPluginCenterRoute()) scheduleSaveBounds(appWindow);
      }).then((unlisten) => {
        unlistenResized = unlisten;
      });
    });
  });

  onUnmounted(() => {
    tearDownDragGuard?.();
    tearDownDragGuard = null;
    unlistenBlur?.();
    unlistenBlur = null;
    unlistenMoved?.();
    unlistenMoved = null;
    unlistenResized?.();
    unlistenResized = null;
    cancelHide();
    cancelSaveBounds();
  });

  return {
    hide: () => getWindow().then((w) => w.hide()),
    show: () => getWindow().then((w) => w.show()),
    setFocus: () => getWindow().then((w) => w.setFocus()),
  };
}

export function useShortcut() {
  if (!getCurrentInstance()) return;
  void getWindow().then((w) => {
    document.addEventListener("keydown", (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        w.hide().catch(() => {});
      }
    });
  });
}
