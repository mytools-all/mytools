import { mount, flushPromises } from "@vue/test-utils";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import PluginManagerView from "@/views/PluginManagerView.vue";
import type { LoadedPlugin } from "@/stores/plugins";
import { useSettingsStore } from "@/stores/settings";

let pluginListOverride: LoadedPlugin[] | null = null;
let dirGrantsOverride: { plugin: string; path: string }[] = [];
let detached = false;

const invokeMock = vi.fn(async (cmd: string, _args?: unknown): Promise<unknown> => {
  if (cmd === "plugin_list") return pluginListOverride ?? pluginFixture();
  if (cmd === "plugin_get_root") return "C:\\mytools\\plugins";
  if (cmd === "dir_grants_list") return dirGrantsOverride;
  return null;
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...(args as [string, unknown])),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
}));

vi.mock("@/lib/tauri", () => ({
  safeInvoke: (...args: unknown[]) => {
    const cmd = args[0] as string;
    const a = args[1] as Record<string, unknown> | undefined;
    return invokeMock(cmd, a);
  },
  tauriAvailable: () => true,
  isDetachedCenter: () => detached,
}));

function officialFixture(): LoadedPlugin[] {
  return [
    {
      manifest: {
        name: "mytools-calc",
        pluginName: "计算稿纸",
        description: "表达式求值与稿纸记录",
        version: "1.0.0",
        main: "index.html",
        features: [{ code: "calc", explain: "计算", cmds: ["计算"] }],
      },
      dir: "C:\\mytools\\plugins\\mytools-calc",
      mainUrl: "http://127.0.0.1:0/x/mytools-calc/index.html",
      icon: "data:image/svg+xml;utf8,<svg/>",
      enabled: true,
      origin: "official",
      declaredCapabilities: [],
    },
    {
      manifest: {
        name: "demo-hello",
        pluginName: "Demo Hello",
        description: "a demo plugin",
        version: "1.0.0",
        main: "index.html",
        features: [{ code: "index", explain: "open", cmds: ["hello"] }],
      },
      dir: "C:\\mytools\\plugins\\demo-hello",
      mainUrl: "http://127.0.0.1:0/x/demo-hello/index.html",
      icon: "data:image/svg+xml;utf8,<svg/>",
      enabled: true,
      origin: "custom",
      declaredCapabilities: [],
    },
  ];
}

function pluginFixture(): LoadedPlugin[] {
  return officialFixture();
}

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/", name: "launcher", component: { template: "<div />" } },
      { path: "/settings", name: "settings", component: { template: "<div />" } },
      { path: "/plugin/:id/:code", name: "plugin", component: { template: "<div />" } },
      { path: "/plugins", name: "plugins", component: PluginManagerView },
    ],
  });
}

async function mountView(section = "installed") {
  const router = makeRouter();
  await router.push({ path: "/plugins", query: { section } });
  await router.isReady();
  const pinia = createPinia();
  setActivePinia(pinia);
  const wrapper = mount(PluginManagerView, {
    global: { plugins: [pinia, router] },
  });
  await flushPromises();
  return { wrapper, router };
}

describe("PluginManagerView (插件中心)", () => {
  beforeEach(() => {
    detached = false;
    setActivePinia(createPinia());
    invokeMock.mockClear();
    pluginListOverride = null;
    dirGrantsOverride = [];
  });

  it("defaults to the store section", async () => {
    const { wrapper } = await mountView("store");
    expect(wrapper.find(".pc__side-nav-item--active").text()).toBe("插件商店");
    expect(wrapper.findAll(".tc__card").length).toBeGreaterThan(0);
  });

  it.each(["dev", "settings"])("removes the detached page search from %s", async (section) => {
    detached = true;
    const {wrapper} = await mountView(section);
    expect(wrapper.find(".pc__main > .pc__search-input").exists()).toBe(false);
    wrapper.unmount();
  });

  it("keeps a dedicated search for detached installed plugins", async () => {
    detached = true;
    const {wrapper} = await mountView("installed");
    await wrapper.get(".pc__installed-search").setValue("计算稿纸");
    expect(wrapper.findAll(".pc__iitem")).toHaveLength(1);
    expect(wrapper.get(".pc__iitem").text()).toContain("计算稿纸");
    wrapper.unmount();
  });

  it("has store, installed and developer sections", async () => {
    const { wrapper } = await mountView("store");
    const labels = wrapper.findAll(".pc__side-nav-item").map((n) => n.text());
    expect(labels).toEqual(["插件商店", "已安装", "开发者", "偏好设置"]);
  });

  it("lists installed plugins with their origin badge", async () => {
    const { wrapper } = await mountView("installed");
    const items = wrapper.findAll(".pc__iitem");
    expect(items.length).toBe(2);
    expect(items[0].text()).toContain("官方");
    expect(items[1].text()).toContain("自定义");
    expect(wrapper.text()).not.toContain("Tier");
  });

  it("disables a plugin from the installed list", async () => {
    const { wrapper } = await mountView("installed");
    const disableBtn = wrapper
      .findAll(".pc__iitem .pc__btn")
      .find((b) => b.text() === "停用")!;
    await disableBtn.trigger("click");
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith("plugin_disable", {
      code: "mytools-calc",
    });
  });

  it("installs a plugin from npm in the developer section", async () => {
    const { wrapper } = await mountView("dev");
    const input = wrapper.find(".pc__search-input");
    await input.setValue("mytools-demo@1.0.0");
    const installBtn = wrapper
      .findAll(".pc__btn")
      .find((b) => b.text() === "从 npm 安装")!;
    await installBtn.trigger("click");
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith("plugin_install_npm", {
      spec: "mytools-demo@1.0.0",
    });
  });

  it("preserves the installed filter and developer draft while switching pages", async () => {
    pluginListOverride = pluginFixture();
    pluginListOverride[1]!.enabled = false;
    const {wrapper} = await mountView("installed");
    await wrapper.findAll(".ttn__tab").find(tab => tab.text().includes("已停用"))!.trigger("click");
    expect(wrapper.findAll(".pc__iitem")).toHaveLength(1);
    await wrapper.get('[data-section="dev"]').trigger("click");
    await flushPromises();
    await wrapper.get(".pc__search-input").setValue("@mytools-all/mytools-demo@0.1.1");
    await wrapper.get('[data-section="installed"]').trigger("click");
    await flushPromises();
    expect(wrapper.get(".ttn__tab--active").text()).toContain("已停用");
    expect(wrapper.get(".pc__iitem").text()).toContain("Demo Hello");
    await wrapper.get('[data-section="dev"]').trigger("click");
    await flushPromises();
    expect((wrapper.get(".pc__search-input").element as HTMLInputElement).value).toBe("@mytools-all/mytools-demo@0.1.1");
    wrapper.unmount();
  });

  it("loads permissions only when preferences is first visited", async () => {
    const {wrapper} = await mountView("store");
    expect(invokeMock).not.toHaveBeenCalledWith("plugin_capability_grants", undefined);
    expect(invokeMock).not.toHaveBeenCalledWith("dir_grants_list", undefined);
    await wrapper.get('[data-section="settings"]').trigger("click");
    await flushPromises();
    expect(invokeMock.mock.calls.filter(([command]) => command === "plugin_capability_grants")).toHaveLength(1);
    expect(invokeMock.mock.calls.filter(([command]) => command === "dir_grants_list")).toHaveLength(1);
    await wrapper.get('[data-section="store"]').trigger("click");
    await flushPromises();
    await wrapper.get('[data-section="settings"]').trigger("click");
    await flushPromises();
    expect(invokeMock.mock.calls.filter(([command]) => command === "plugin_capability_grants")).toHaveLength(1);
    wrapper.unmount();
  });

  it("opens plugins in their own window when the center is detached", async () => {
    detached = true;
    const {wrapper,router} = await mountView("installed");
    await wrapper.findAll(".pc__iactions .pc__btn").find(button => button.text() === "打开")!.trigger("click");
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith("plugin_open_window", {pluginId:"mytools-calc",feature:"calc"});
    expect(router.currentRoute.value.path).toBe("/plugins");
    expect(invokeMock.mock.calls.some(([command]) => command === "window_set_layout")).toBe(false);
    wrapper.unmount();
  });

  it("opens and closes the detail drawer from the installed page", async () => {
    const {wrapper,router} = await mountView("installed");
    await wrapper.get(".pc__iitem").trigger("click");
    await flushPromises();
    expect(router.currentRoute.value.query.detail).toBe("mytools-calc");
    expect(wrapper.get(".pc").attributes("aria-hidden")).toBe("true");
    const close = document.querySelector<HTMLButtonElement>('[aria-label="关闭插件详情"]');
    expect(close).not.toBeNull();
    close!.click();
    await flushPromises();
    expect(router.currentRoute.value.query.detail).toBeUndefined();
    expect(wrapper.get(".pc").attributes("aria-hidden")).toBeUndefined();
    wrapper.unmount();
  });

  it("lists directory grants in the settings section", async () => {
    dirGrantsOverride = [{ plugin: "mytools-calc", path: "C:\\work" }];
    const { wrapper } = await mountView("settings");
    useSettingsStore().settingsCategory = "data";
    await flushPromises();
    expect(wrapper.text()).toContain("C:\\work");
  });

  it("revokes a directory grant from the settings page", async () => {
    dirGrantsOverride = [{ plugin: "mytools-calc", path: "C:\\work" }];
    const { wrapper } = await mountView("settings");
    useSettingsStore().settingsCategory = "data";
    await flushPromises();
    const revoke = wrapper
      .findAll(".grants__revoke")
      .find((b) => b.text() === "撤销")!;
    await revoke.trigger("click");
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith("dir_grants_revoke", {
      plugin: "mytools-calc",
      path: "C:\\work",
    });
  });

  it("grows the window on mount and restores it on unmount", async () => {
    const { wrapper } = await mountView("tools");
    expect(invokeMock).toHaveBeenCalledWith("window_set_layout", {
      resizable: true,
      decorations: false,
      width: 1024,
      height: 760,
    });
    wrapper.unmount();
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith("window_set_layout", {
      resizable: true,
      decorations: false,
      width: 720,
      height: 480,
    });
  });
});
