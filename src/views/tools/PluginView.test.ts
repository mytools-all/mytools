import { reactive } from "vue";
import type { Window as HappyWindow } from "happy-dom";
import { mount, flushPromises } from "@vue/test-utils";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory, type Router } from "vue-router";
import PluginView from "@/views/tools/PluginView.vue";
import {
  clearPluginErrors,
  reportPluginError,
  usePluginErrors,
} from "@/composables/pluginBridge";

(window as unknown as HappyWindow).happyDOM.settings.disableIframePageLoading = true;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => null),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

const refreshPlugins = vi.hoisted(() => vi.fn(async () => {}));

const stubPlugins = reactive([
  {
    manifest: {
      name: "demo",
      pluginName: "示例",
      description: "",
      version: "0.0.1",
      main: "index.html",
      features: [{ code: "run", explain: "运行", cmds: [] }],
    },
    dir: "/tmp/demo",
    mainUrl: "http://127.0.0.1:1234/token/demo/index.html",
    enabled: true,
  },
]);

vi.mock("@/stores/plugins", () => ({
  usePluginsStore: () => ({
    plugins: stubPlugins,
    refresh: refreshPlugins,
    ensureLoaded: refreshPlugins,
  }),
}));

function makeRouter(): Router {
  return createRouter({
    history: createMemoryHistory(),
    routes: [{ path: "/plugin/:id/:code", component: PluginView }],
  });
}

describe("PluginView", () => {
  beforeEach(() => {
    refreshPlugins.mockReset();
    refreshPlugins.mockResolvedValue();
    stubPlugins[0]!.enabled = true;
    vi.spyOn(console, "error").mockImplementation(() => {});
    vi.spyOn(HTMLIFrameElement.prototype, "contentWindow", "get").mockReturnValue({ postMessage: vi.fn() } as unknown as Window);
    setActivePinia(createPinia());
    clearPluginErrors("demo:run");
  });

  it("loads the iframe from mainUrl without appending a hash", async () => {
    const router = makeRouter();
    router.push("/plugin/demo/run");
    await router.isReady();
    const wrapper = mount(PluginView, {
      global: { plugins: [router] },
      attachTo: document.body,
    });
    await flushPromises();
    const iframe = wrapper.find("iframe.pv__frame");
    expect(iframe.exists()).toBe(true);
    expect(iframe.attributes("referrerpolicy")).toBe("origin");
    expect(iframe.attributes("src")).toBe("http://127.0.0.1:1234/token/demo/index.html");
    wrapper.unmount();
  });

  it("sends ready and enter into the iframe once it loads", async () => {
    const router = makeRouter();
    router.push("/plugin/demo/run");
    await router.isReady();
    const wrapper = mount(PluginView, {
      global: { plugins: [router] },
      attachTo: document.body,
    });
    await flushPromises();
    const iframe = wrapper.find("iframe.pv__frame");
    const post = vi.spyOn(
      (iframe.element as HTMLIFrameElement).contentWindow as Window,
      "postMessage",
    );
    await iframe.trigger("load");
    expect(post).toHaveBeenCalledWith(
      {
        $mytoolsEvent: {
          event: "ready",
          payload: { code: "run", type: "text", plugin: "demo", apiVersion: 1, hostSearch: false },
        },
      },
      "http://127.0.0.1:1234",
    );
    expect(post).toHaveBeenCalledWith(
      {
        $mytoolsEvent: {
          event: "enter",
          payload: { code: "run", type: "text", plugin: "demo", apiVersion: 1, hostSearch: false },
        },
      },
      "http://127.0.0.1:1234",
    );
    wrapper.unmount();
  });

  it("re-enters the plugin when the feature code changes", async () => {
    const router = makeRouter();
    router.push("/plugin/demo/run");
    await router.isReady();
    const wrapper = mount(PluginView, {
      global: { plugins: [router] },
      attachTo: document.body,
    });
    await flushPromises();
    const iframe = wrapper.find("iframe.pv__frame");
    await iframe.trigger("load");
    const post = vi.spyOn(
      (iframe.element as HTMLIFrameElement).contentWindow as Window,
      "postMessage",
    );
    await router.push("/plugin/demo/other");
    await flushPromises();
    expect(post).toHaveBeenCalledWith(
      {
        $mytoolsEvent: {
          event: "enter",
          payload: { code: "other", type: "text", plugin: "demo", apiVersion: 1, hostSearch: false },
        },
      },
      "http://127.0.0.1:1234",
    );
    const src = iframe.attributes("src");
    expect(src).toBe("http://127.0.0.1:1234/token/demo/index.html");
    wrapper.unmount();
  });

  it("unmounts a running plugin when it is disabled", async () => {
    const router = makeRouter();
    await router.push("/plugin/demo/run");
    const wrapper = mount(PluginView, { global: { plugins: [router] } });
    await flushPromises();
    expect(wrapper.find("iframe").exists()).toBe(true);
    stubPlugins[0]!.enabled = false;
    await flushPromises();
    expect(wrapper.find("iframe").exists()).toBe(false);
    expect(wrapper.find('[role="alert"]').exists()).toBe(true);
    wrapper.unmount();
  });

  it("shows a load failure instead of waiting forever", async () => {
    refreshPlugins.mockRejectedValueOnce(new Error("scan failed"));
    const router = makeRouter();
    await router.push("/plugin/demo/run");
    const wrapper = mount(PluginView, { global: { plugins: [router] } });
    await flushPromises();
    expect(wrapper.find('[role="alert"]').text()).toBe("scan failed");
    expect(wrapper.find("iframe").exists()).toBe(false);
    wrapper.unmount();
  });

  it("does not mount an obsolete plugin after a delayed refresh", async () => {
    let finish: (() => void) | undefined;
    refreshPlugins.mockImplementationOnce(() => new Promise<void>((resolve) => { finish = resolve; }));
    const router = makeRouter();
    await router.push("/plugin/demo/run");
    const wrapper = mount(PluginView, { global: { plugins: [router] } });
    await router.push("/plugin/missing/run");
    await flushPromises();
    finish?.();
    await flushPromises();
    expect(wrapper.find("iframe").exists()).toBe(false);
    expect(wrapper.find('[role="alert"]').exists()).toBe(true);
    wrapper.unmount();
  });

  it("renders the error panel when a $mytools error is reported", async () => {
    const router = makeRouter();
    router.push("/plugin/demo/run");
    await router.isReady();
    const wrapper = mount(PluginView, {
      global: { plugins: [router] },
      attachTo: document.body,
    });
    await flushPromises();
    reportPluginError("demo:run", {
      kind: "error",
      message: "插件崩了",
      stack: "at line 1\nat line 2",
    });
    await flushPromises();
    const panel = wrapper.find(".pv__errors");
    expect(panel.exists()).toBe(true);
    expect(panel.text()).toContain("插件崩了");
    expect(panel.text()).toContain("at line 1");
    wrapper.unmount();
  });

  it("hides the error panel after errors are cleared", async () => {
    const router = makeRouter();
    router.push("/plugin/demo/run");
    await router.isReady();
    const wrapper = mount(PluginView, {
      global: { plugins: [router] },
      attachTo: document.body,
    });
    await flushPromises();
    reportPluginError("demo:run", { kind: "error", message: "x" });
    await flushPromises();
    expect(wrapper.find(".pv__errors").exists()).toBe(true);
    usePluginErrors().clear("demo:run");
    await flushPromises();
    expect(wrapper.find(".pv__errors").exists()).toBe(false);
    wrapper.unmount();
  });

  it("does not render the error panel for ResizeObserver loop noise", async () => {
    const router = makeRouter();
    router.push("/plugin/demo/run");
    await router.isReady();
    const wrapper = mount(PluginView, {
      global: { plugins: [router] },
      attachTo: document.body,
    });
    await flushPromises();
    reportPluginError("demo:run", {
      kind: "error",
      message: "ResizeObserver loop completed with undelivered notifications.",
    });
    await flushPromises();
    expect(wrapper.find(".pv__errors").exists()).toBe(false);
    wrapper.unmount();
  });
});
