import { listen } from "@tauri-apps/api/event";
import { askCapabilityConsent, resolveCapabilityConsent } from "@/runtime/capabilityConsent";
import { mount, flushPromises } from "@vue/test-utils";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import App from "@/App.vue";
import { useSettingsStore } from "@/stores/settings";

const {
  invokeMock,
  focusHandler,
  movedHandler,
  resizedHandler,
  hideMock,
  onFocusChangedMock,
  onMovedMock,
  onResizedMock,
  outerPositionMock,
  outerSizeMock,
  appIndexListResult,
} = vi.hoisted(() => {
  const appIndexListResult: { value: unknown } = { value: [] };
  const invokeMock = vi.fn(async (cmd: string, _args?: unknown) => {
    if (cmd === "plugin_list") return [];
    if (cmd === "plugin_get_root") return "";
    if (cmd === "app_index_list") return appIndexListResult.value;
    if (cmd === "app_index_refresh_if_stale") return false;
    if (cmd === "app_resolve") return null;
    return "pong";
  });
  const hideMock = vi.fn(async () => {});
  const focusHandler: { current: ((event: { payload: boolean }) => void) | null } = {
    current: null,
  };
  const movedHandler: {
    current: ((event: { payload: unknown }) => void) | null;
  } = { current: null };
  const resizedHandler: {
    current: ((event: { payload: unknown }) => void) | null;
  } = { current: null };
  const onFocusChangedMock = vi.fn(
    async (cb: (event: { payload: boolean }) => void) => {
      (globalThis as { __LAST_CB__?: unknown }).__LAST_CB__ = cb;
      focusHandler.current = cb;
      return () => {
        focusHandler.current = null;
      };
    },
  );
  const onMovedMock = vi.fn(
    async (cb: (event: { payload: unknown }) => void) => {
      movedHandler.current = cb;
      return () => {
        movedHandler.current = null;
      };
    },
  );
  const onResizedMock = vi.fn(
    async (cb: (event: { payload: unknown }) => void) => {
      resizedHandler.current = cb;
      return () => {
        resizedHandler.current = null;
      };
    },
  );
  const outerPositionMock = vi.fn(async () => ({ x: 120, y: 80 }));
  const outerSizeMock = vi.fn(async () => ({ width: 900, height: 600 }));
  return {
    invokeMock,
    focusHandler,
    movedHandler,
    resizedHandler,
    hideMock,
    onFocusChangedMock,
    onMovedMock,
    onResizedMock,
    outerPositionMock,
    outerSizeMock,
    appIndexListResult,
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...(args as [string, unknown])),
}));

const eventListeners: Record<string, (event: { payload: string }) => void> = {};

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (event: string, handler: (event: { payload: string }) => void) => {
    eventListeners[event] = handler;
    return () => {};
  }),
}));

vi.mock("@/lib/tauri", () => ({
  safeInvoke: (...args: unknown[]) => {
    const cmd = args[0] as string;
    const a = args[1] as Record<string, unknown> | undefined;
    return invokeMock(cmd, a);
  },
  tauriAvailable: () => true,
}));

let windowLabel = "main";

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({ label: windowLabel, listen: vi.fn(async () => () => {}) }),
}));

vi.mock("@tauri-apps/api/window", () => {
  return {
    getCurrentWindow: () => {
      return {
        onFocusChanged: onFocusChangedMock,
        onMoved: onMovedMock,
        onResized: onResizedMock,
        hide: hideMock,
        show: vi.fn(async () => {}),
        setFocus: vi.fn(async () => {}),
        outerPosition: outerPositionMock,
        outerSize: outerSizeMock,
      };
    },
  };
});

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/", name: "launcher", component: () => import("@/views/LauncherView.vue") },
      { path: "/tool/json-formatter", name: "json-formatter", component: () => import("@/views/tools/JsonFormatterView.vue") },
      { path: "/tool/base64-url", name: "base64-url", component: () => import("@/views/tools/Base64UrlView.vue") },
      { path: "/tool/timestamp-uuid", name: "timestamp-uuid", component: () => import("@/views/tools/TimestampUuidView.vue") },
      { path: "/tool/color-picker", name: "color-picker", component: () => import("@/views/tools/ColorPickerView.vue") },
      { path: "/tool/clipboard", name: "clipboard", component: () => import("@/views/tools/ClipboardView.vue") },
      { path: "/plugin/:id/:code", name: "plugin", component: () => import("@/views/tools/PluginView.vue") },
      { path: "/plugins", name: "plugins", component: () => import("@/views/PluginManagerView.vue") },
    ],
  });
}

describe("App.vue", () => {
  beforeEach(() => {
    for (const key of Object.keys(eventListeners)) delete eventListeners[key];
    setActivePinia(createPinia());
    invokeMock.mockClear();
    hideMock.mockClear();
    onMovedMock.mockClear();
    onResizedMock.mockClear();
    outerPositionMock.mockClear();
    outerSizeMock.mockClear();
    focusHandler.current = null;
    movedHandler.current = null;
    resizedHandler.current = null;
    appIndexListResult.value = [];
    windowLabel = "main";
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    window.location.hash = "";
  });

  it("renders the launcher view by default with global header", async () => {
    const router = makeRouter();
    router.push("/");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    expect(wrapper.find(".app-header").exists()).toBe(true);
    expect(wrapper.find(".search-box").exists()).toBe(true);
  });

  it("app header is a draggable region", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" } },
      transformCallback: () => 0,
      invoke: invokeMock,
    };
    const router = makeRouter();
    router.push("/");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    expect(wrapper.find(".app-header").attributes("data-tauri-drag-region")).toBe("deep");
  });

  it("renders the JSON formatter view at the right path", async () => {
    const router = makeRouter();
    router.push("/tool/json-formatter");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    expect(wrapper.find(".json__panes").exists()).toBe(true);
  });

  it("renders the Base64/URL view", async () => {
    const router = makeRouter();
    router.push("/tool/base64-url");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    expect(wrapper.find(".b64__panes").exists()).toBe(true);
  });

  it("renders the Timestamp/UUID view", async () => {
    const router = makeRouter();
    router.push("/tool/timestamp-uuid");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    expect(wrapper.find(".tu__h3").exists()).toBe(true);
  });

  it("renders the Clipboard view", async () => {
    const router = makeRouter();
    router.push("/tool/clipboard");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    expect(wrapper.find(".cb").exists()).toBe(true);
  });

  it("renders the PluginView at /plugin/:id/:code", async () => {
    const router = makeRouter();
    router.push("/plugin/my-plugin/index");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    expect(wrapper.find(".pv").exists()).toBe(true);
  });

  it("renders the plugin center view at /plugins with global header", async () => {
    const router = makeRouter();
    router.push("/plugins");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    expect(wrapper.find(".app-header").exists()).toBe(true);
    expect(wrapper.find(".pc").exists()).toBe(true);
  });

  it("the header has no version label and no window-close button", async () => {
    const router = makeRouter();
    router.push("/");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    expect(wrapper.find(".app-header__version").exists()).toBe(false);
    expect(wrapper.find(".app-header__close").exists()).toBe(false);
    expect(wrapper.text()).not.toContain("v0.1.0");
  });

  it("renders a detached plugin window with no header at all", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "plugin-rubick-markdown-1" } },
      transformCallback: () => 0,
      invoke: invokeMock,
    };
    windowLabel = "plugin-rubick-markdown-1";
    const router = makeRouter();
    router.push("/plugin/rubick-markdown/editor");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    await flushPromises();
    // Bare plugin surface: no header, no search box, no resize handles —
    // only the native window decorations.
    expect(wrapper.find(".app-header").exists()).toBe(false);
    expect(wrapper.find(".search-box").exists()).toBe(false);
    expect(wrapper.find(".pv").exists()).toBe(true);
    wrapper.unmount();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    windowLabel = "main";
  });

  it("mounts the webview health monitor on first render", async () => {
    const router = makeRouter();
    router.push("/");
    await router.isReady();
    mount(App, { global: { plugins: [router] } });
    await flushPromises();
    const calls = invokeMock.mock.calls.map((c) => c[0]);
    expect(calls).toContain("webview_health");
  });

  it("tray settings shows the window before navigating to /plugins?section=settings", async () => {
    const router = makeRouter();
    router.push("/");
    await router.isReady();
    mount(App, { global: { plugins: [router] } });
    await flushPromises();
    await eventListeners["tray:menu"]({ payload: "settings" });
    await flushPromises();
    const windowCalls = invokeMock.mock.calls.filter((c) => c[0] === "window_show");
    expect(windowCalls.length).toBeGreaterThan(0);
    expect(router.currentRoute.value.name).toBe("plugins");
    expect(router.currentRoute.value.query.section).toBe("settings");
  });

  it("releases the tray listener when the app is unmounted", async () => {
    const unlisten = vi.fn();
    vi.mocked(listen).mockResolvedValueOnce(unlisten);
    const router = makeRouter();
    await router.push("/");
    const wrapper = mount(App, { global: { plugins: [router] } });
    await flushPromises();
    wrapper.unmount();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("releases a tray subscription that finishes after unmount", async () => {
    let finish: ((value: () => void) => void) | undefined;
    const unlisten = vi.fn();
    vi.mocked(listen).mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; }));
    const router = makeRouter();
    await router.push("/");
    const wrapper = mount(App, { global: { plugins: [router] } });
    await flushPromises();
    expect(finish).toBeDefined();
    wrapper.unmount();
    finish?.(unlisten);
    await flushPromises();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("shows capability consent in detached plugin windows", async () => {
    windowLabel = "plugin-demo-1";
    const router = makeRouter();
    await router.push("/");
    const wrapper = mount(App, { global: { plugins: [router] } });
    await flushPromises();
    const pending = askCapabilityConsent({ plugin: "demo", capability: "fs.write", message: "Allow?" });
    await flushPromises();
    expect(wrapper.find('[data-testid="capability-consent"]').exists()).toBe(true);
    resolveCapabilityConsent(false);
    await expect(pending).resolves.toBe(false);
    wrapper.unmount();
  });

  it("renders the floatball view in a floatball window", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "floatball" } },
      transformCallback: () => 0,
      invoke: invokeMock,
    };
    windowLabel = "floatball";
    const router = makeRouter();
    router.push("/");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    await flushPromises();
    expect(wrapper.find(".floatball__ball").exists()).toBe(true);
    expect(wrapper.find(".app-header").exists()).toBe(false);
    wrapper.unmount();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it("renders the router when window label is main", async () => {
    const router = makeRouter();
    router.push("/");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    await flushPromises();
    expect(wrapper.find(".floatball").exists()).toBe(false);
  });

  it("ignores tray events in non-main windows", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "floatball" } },
      transformCallback: () => 0,
      invoke: invokeMock,
    };
    windowLabel = "floatball";
    const router = makeRouter();
    router.push("/");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    await flushPromises();
    invokeMock.mockClear();
    expect(eventListeners["tray:menu"]).toBeUndefined();
    await flushPromises();
    const windowCalls = invokeMock.mock.calls.filter((c) => c[0] === "window_show");
    expect(windowCalls).toHaveLength(0);
    wrapper.unmount();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    windowLabel = "main";
  });

  it("hides the main window on blur after the debounce (click outside)", async () => {
    vi.useFakeTimers();
    try {
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: "main" } },
        transformCallback: () => 0,
        invoke: invokeMock,
      };
      const router = makeRouter();
      router.push("/");
      await router.isReady();
      mount(App, { global: { plugins: [router] } });
      await flushPromises();
      expect(focusHandler.current).toBeTruthy();
      focusHandler.current?.({ payload: false });
      await vi.advanceTimersByTimeAsync(250);
      expect(hideMock).not.toHaveBeenCalled();
      await vi.advanceTimersByTimeAsync(100);
      expect(hideMock).toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });

  it("keeps a plugin visible when switching to Explorer to drag files", async () => {
    vi.useFakeTimers();
    try {
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
        metadata: {currentWindow:{label:"main"}}, transformCallback: () => 0, invoke: invokeMock,
      };
      const router = makeRouter();
      await router.push("/plugin/fixture/main");
      await router.isReady();
      const wrapper = mount(App, {global:{plugins:[router]}});
      await flushPromises();
      focusHandler.current?.({payload:false});
      await vi.advanceTimersByTimeAsync(500);
      expect(hideMock).not.toHaveBeenCalled();
      wrapper.unmount();
    } finally { vi.useRealTimers(); }
  });

  it("does not hide on blur while a drag-region is being dragged", async () => {
    vi.useFakeTimers();
    try {
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: "main" } },
        transformCallback: () => 0,
        invoke: invokeMock,
      };
      const router = makeRouter();
      router.push("/");
      await router.isReady();
      mount(App, { global: { plugins: [router] } });
      await flushPromises();
      expect(focusHandler.current).toBeTruthy();
      const header = document.createElement("header");
      header.setAttribute("data-tauri-drag-region", "deep");
      document.body.appendChild(header);
      header.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
      focusHandler.current?.({ payload: false });
      await vi.advanceTimersByTimeAsync(600);
      expect(hideMock).not.toHaveBeenCalled();
      document.body.removeChild(header);
    } finally {
      vi.useRealTimers();
    }
  });

  it("hides on blur again once the pointer is released", async () => {
    vi.useFakeTimers();
    try {
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: "main" } },
        transformCallback: () => 0,
        invoke: invokeMock,
      };
      const router = makeRouter();
      router.push("/");
      await router.isReady();
      mount(App, { global: { plugins: [router] } });
      await flushPromises();
      expect(focusHandler.current).toBeTruthy();
      const header = document.createElement("header");
      header.setAttribute("data-tauri-drag-region", "deep");
      document.body.appendChild(header);
      header.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
      document.dispatchEvent(new PointerEvent("pointerup", { bubbles: true }));
      focusHandler.current?.({ payload: false });
      await vi.advanceTimersByTimeAsync(400);
      expect(hideMock).toHaveBeenCalled();
      document.body.removeChild(header);
    } finally {
      vi.useRealTimers();
    }
  });

  it("does not hide on blur when the window moves (native drag)", async () => {
    vi.useFakeTimers();
    try {
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: "main" } },
        transformCallback: () => 0,
        invoke: invokeMock,
      };
      const router = makeRouter();
      router.push("/");
      await router.isReady();
      mount(App, { global: { plugins: [router] } });
      await flushPromises();
      expect(focusHandler.current).toBeTruthy();
      expect(movedHandler.current).toBeTruthy();
      focusHandler.current?.({ payload: false });
      movedHandler.current?.({ payload: { x: 640, y: 360 } });
      await vi.advanceTimersByTimeAsync(500);
      expect(hideMock).not.toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });

  it("does not hide on blur when the window is being resized", async () => {
    vi.useFakeTimers();
    try {
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: "main" } },
        transformCallback: () => 0,
        invoke: invokeMock,
      };
      const router = makeRouter();
      router.push("/");
      await router.isReady();
      mount(App, { global: { plugins: [router] } });
      await flushPromises();
      expect(focusHandler.current).toBeTruthy();
      expect(resizedHandler.current).toBeTruthy();
      focusHandler.current?.({ payload: false });
      resizedHandler.current?.({ payload: { width: 1000, height: 700 } });
      await vi.advanceTimersByTimeAsync(500);
      expect(hideMock).not.toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });

  it("persists main window bounds after move or resize", async () => {
    vi.useFakeTimers();
    try {
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: "main" } },
        transformCallback: () => 0,
        invoke: invokeMock,
      };
      const router = makeRouter();
      router.push("/");
      await router.isReady();
      mount(App, { global: { plugins: [router] } });
      await flushPromises();
      expect(movedHandler.current).toBeTruthy();
      movedHandler.current?.({ payload: { x: 640, y: 360 } });
      await vi.advanceTimersByTimeAsync(500);
      const saveCalls = invokeMock.mock.calls.filter(
        (c) => c[0] === "window_save_bounds",
      );
      expect(saveCalls.length).toBeGreaterThan(0);
      expect(saveCalls[0][1]).toMatchObject({ x: 120, y: 80, width: 900, height: 600 });
    } finally {
      vi.useRealTimers();
    }
  });

  it("registers moved/resized listeners only for the main window", async () => {
    vi.useFakeTimers();
    try {
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: "floatball" } },
        transformCallback: () => 0,
        invoke: invokeMock,
      };
      windowLabel = "floatball";
      const router = makeRouter();
      router.push("/");
      await router.isReady();
      mount(App, { global: { plugins: [router] } });
      await flushPromises();
      expect(focusHandler.current).toBeNull();
      expect(movedHandler.current).toBeNull();
      expect(resizedHandler.current).toBeNull();
      delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    } finally {
      vi.useRealTimers();
    }
  });

  it("floatball window never registers a blur hide", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "floatball" } },
      transformCallback: () => 0,
      invoke: invokeMock,
    };
    windowLabel = "floatball";
    const router = makeRouter();
    router.push("/");
    await router.isReady();
    mount(App, { global: { plugins: [router] } });
    await flushPromises();
    expect(focusHandler.current).toBeNull();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it("does not persist bounds while on the plugin-center route", async () => {
    vi.useFakeTimers();
    try {
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: "main" } },
        transformCallback: () => 0,
        invoke: invokeMock,
      };
      window.location.hash = "#/plugins";
      const router = makeRouter();
      router.push("/");
      await router.isReady();
      mount(App, { global: { plugins: [router] } });
      await flushPromises();
      expect(resizedHandler.current).toBeTruthy();
      resizedHandler.current?.({ payload: { width: 1024, height: 760 } });
      movedHandler.current?.({ payload: { x: 10, y: 10 } });
      await vi.advanceTimersByTimeAsync(600);
      const saveCalls = invokeMock.mock.calls.filter(
        (c) => c[0] === "window_save_bounds",
      );
      expect(saveCalls.length).toBe(0);
    } finally {
      window.location.hash = "";
      vi.useRealTimers();
    }
  });

  it("refreshes the app index when the main window regains focus", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" } },
      transformCallback: () => 0,
      invoke: invokeMock,
    };
    const router = makeRouter();
    router.push("/");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    await flushPromises();
    expect(focusHandler.current).toBeTruthy();
    invokeMock.mockClear();
    focusHandler.current?.({ payload: true });
    await flushPromises();
    const calls = invokeMock.mock.calls.map((c) => c[0]);
    expect(calls).toContain("app_index_refresh_if_stale");
    wrapper.unmount();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it("shows pinned app tiles with the indexed display name", async () => {
    const target =
      "windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel";
    appIndexListResult.value = [{ name: "设置", target, subtitle: "app (system)" }];
    useSettingsStore().pinnedPlugins = [`app:${target}`];
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" } },
      transformCallback: () => 0,
      invoke: invokeMock,
    };
    const router = makeRouter();
    router.push("/");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    await flushPromises();
    await flushPromises();
    const names = wrapper.findAll(".ptiles__name").map((n) => n.text());
    expect(names).toContain("设置");
    expect(wrapper.text()).not.toContain("windows.immersivecontrolpanel");
    wrapper.unmount();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it("shows legacy ms-settings pins as the Settings app", async () => {
    useSettingsStore().pinnedPlugins = ["app:ms-settings:display"];
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" } },
      transformCallback: () => 0,
      invoke: invokeMock,
    };
    const router = makeRouter();
    router.push("/");
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });
    await flushPromises();
    await flushPromises();
    expect(wrapper.findAll(".ptiles__name").map((n) => n.text())).toContain("设置");
    expect(wrapper.text()).not.toContain("ms-settings");
    wrapper.unmount();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });
});
