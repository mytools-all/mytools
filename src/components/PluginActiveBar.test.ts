import { mount, flushPromises } from "@vue/test-utils";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import PluginActiveBar from "@/components/PluginActiveBar.vue";
import { useActivePluginStore } from "@/stores/activePlugin";

const pluginsMock = {
  plugins: [] as Array<{
    manifest: { name: string; pluginName?: string; description: string; version: string; features: unknown[] };
    icon?: string;
  }>,
};

vi.mock("@/stores/plugins", () => ({
  usePluginsStore: () => pluginsMock,
}));

describe("PluginActiveBar", () => {
  beforeEach(() => {
    pluginsMock.plugins = [];
  });

  async function mountBar(path = "/") {
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: "/", name: "launcher", component: { template: "<div />" } },
        { path: "/plugins", name: "plugins", component: { template: "<div />" } },
        { path: "/plugin/:id/:code", name: "plugin", component: { template: "<div />" } },
      ],
    });
    await router.push(path);
    await router.isReady();
    const active = useActivePluginStore();
    const wrapper = mount(PluginActiveBar, { global: { plugins: [pinia, router] } });
    return { wrapper, router, active };
  }

  it("renders nothing on the launcher route", async () => {
    const { wrapper } = await mountBar("/");
    expect(wrapper.find(".pab").exists()).toBe(false);
  });

  it("shows the plugin name on the plugin route", async () => {
    const { wrapper, active } = await mountBar("/plugin/demo/index");
    active.set({ id: "demo", name: "demo", code: "index", title: "示例插件" });
    await wrapper.vm.$nextTick();
    expect(wrapper.find(".pab").exists()).toBe(true);
    expect(wrapper.find(".pab__name").text()).toBe("示例插件");
  });

  it("prefers the loaded plugin icon", async () => {
    const { wrapper, active } = await mountBar("/plugin/demo/index");
    pluginsMock.plugins = [
      {
        manifest: {
          name: "demo",
          pluginName: "示例插件",
          description: "",
          version: "1",
          features: [],
        },
        icon: "data:image/svg+xml;utf8,<svg/>",
      },
    ];
    active.set({ id: "demo", name: "demo", code: "index", title: "示例插件" });
    await wrapper.vm.$nextTick();
    expect(wrapper.find(".pab__icon img").exists()).toBe(true);
  });

  it("falls back to a glyph when there is no icon", async () => {
    const { wrapper, active } = await mountBar("/plugin/demo/index");
    active.set({ id: "demo", name: "demo", code: "index", title: "无图标" });
    await wrapper.vm.$nextTick();
    expect(wrapper.find(".pab__icon-fallback").exists()).toBe(true);
  });

  it("shows 插件商店 on the default plugins route", async () => {
    const { wrapper } = await mountBar("/plugins");
    expect(wrapper.find(".pab__name").text()).toBe("插件商店");
  });

  it("shows 偏好设置 on the settings section", async () => {
    const { wrapper } = await mountBar("/plugins?section=settings");
    expect(wrapper.find(".pab__name").text()).toBe("偏好设置");
  });

  it("shows 已安装插件 / 开发者 for their sections", async () => {
    const installed = await mountBar("/plugins?section=installed");
    expect(installed.wrapper.find(".pab__name").text()).toBe("已安装插件");
    const dev = await mountBar("/plugins?section=dev");
    expect(dev.wrapper.find(".pab__name").text()).toBe("开发者");
  });

  it("× clears the active plugin and navigates to the launcher", async () => {
    const { wrapper, router, active } = await mountBar("/plugin/demo/index");
    active.set({ id: "demo", name: "demo", code: "index", title: "示例插件" });
    await wrapper.vm.$nextTick();
    await wrapper.find(".pab__close").trigger("click");
    await flushPromises();
    expect(active.plugin).toBeNull();
    expect(router.currentRoute.value.name).toBe("launcher");
  });

  it("× on the settings chip returns to the launcher", async () => {
    const { wrapper, router } = await mountBar("/plugins?section=settings");
    await wrapper.find(".pab__close").trigger("click");
    await flushPromises();
    expect(router.currentRoute.value.name).toBe("launcher");
  });
});