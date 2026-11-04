import { mount, flushPromises } from "@vue/test-utils";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import MenuDropdown from "@/components/MenuDropdown.vue";
import { useActivePluginStore } from "@/stores/activePlugin";
import { useSearchStore } from "@/stores/search";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => null),
}));

vi.mock("@/tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

vi.mock("@/lib/tauri", () => ({
  safeInvoke: vi.fn(async () => null),
  tauriAvailable: () => false,
}));

describe("MenuDropdown", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  async function mountMenu(path = "/") {
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
    const search = useSearchStore();
    const wrapper = mount(MenuDropdown, { global: { plugins: [pinia, router] } });
    return { wrapper, router, active, search };
  }

  it("renders the always-present items", async () => {
    const { wrapper } = await mountMenu();
    const labels = wrapper.findAll(".md__item-label").map((n) => n.text());
    expect(labels).toContain("插件商店");
    expect(labels).toContain("偏好设置");
    expect(labels).toContain("分离窗口");
    expect(labels).toContain("退出");
  });

  it("hides the plugin info block when no plugin is active", async () => {
    const { wrapper } = await mountMenu();
    expect(wrapper.find(".md__plugin-info").exists()).toBe(false);
    expect(wrapper.text()).not.toContain("返回主页");
  });

  it("shows the active plugin info and 返回主页 when active", async () => {
    const { wrapper, active } = await mountMenu();
    active.set({ id: "demo", name: "demo", code: "index", title: "示例插件" });
    await wrapper.vm.$nextTick();
    expect(wrapper.find(".md__plugin-info").text()).toContain("示例插件");
    expect(wrapper.text()).toContain("返回主页");
  });

  it("返回主页 clears the active plugin", async () => {
    const { wrapper, active } = await mountMenu();
    active.set({ id: "demo", name: "demo", code: "index", title: "示例插件" });
    await wrapper.vm.$nextTick();
    const back = wrapper.findAll(".md__item").find((b) => b.text().includes("返回主页"))!;
    await back.trigger("click");
    expect(active.plugin).toBeNull();
  });

  it("插件商店 navigates to /plugins?section=store", async () => {
    const { wrapper, router } = await mountMenu();
    const item = wrapper.findAll(".md__item").find((b) => b.text().includes("插件商店"))!;
    await item.trigger("click");
    await flushPromises();
    expect(router.currentRoute.value.query.section).toBe("store");
  });

  it("偏好设置 navigates to /plugins?section=settings", async () => {
    const { wrapper, router } = await mountMenu();
    const item = wrapper.findAll(".md__item").find((b) => b.text().includes("偏好设置"))!;
    await item.trigger("click");
    await flushPromises();
    expect(router.currentRoute.value.query.section).toBe("settings");
  });

  it("disables 分离窗口 when the launcher has no plugin selected", async () => {
    const { wrapper } = await mountMenu();
    const item = wrapper.findAll(".md__item").find((b) => b.text().includes("分离窗口"))!;
    expect(item.attributes("disabled")).toBeDefined();
    expect(item.text()).toContain("无可分离");
  });

  it("enables 分离窗口 on the plugin route", async () => {
    const { wrapper, active } = await mountMenu("/plugin/demo/index");
    active.set({ id: "demo", name: "demo", code: "index", title: "示例插件" });
    await wrapper.vm.$nextTick();
    const item = wrapper.findAll(".md__item").find((b) => b.text().includes("分离窗口"))!;
    expect(item.attributes("disabled")).toBeUndefined();
  });

  it.each(["store", "installed", "dev", "settings"])("detaches plugin center section %s", async (section) => {
    const {wrapper, router} = await mountMenu(`/plugins?section=${section}`);
    const item = wrapper.findAll(".md__item").find(button => button.text().includes("分离窗口"))!;
    expect(item.attributes("disabled")).toBeUndefined();
    await item.trigger("click");
    await flushPromises();
    const {safeInvoke} = await import("@/lib/tauri");
    expect(safeInvoke).toHaveBeenCalledWith("plugin_center_open_window", {section});
    expect(router.currentRoute.value.name).toBe("launcher");
  });

  it("emits close when an item is clicked", async () => {
    const { wrapper } = await mountMenu();
    const item = wrapper.findAll(".md__item").find((b) => b.text().includes("插件商店"))!;
    await item.trigger("click");
    expect(wrapper.emitted("close")).toBeTruthy();
  });

  it("clears the search box after detaching from the launcher", async () => {
    const { wrapper, search } = await mountMenu();
    search.results = [
      { kind: "plugin", id: "demo:index", title: "示例插件", subtitle: "", score: 1 },
    ];
    search.selectedIndex = 0;
    search.query = "示例";
    await wrapper.vm.$nextTick();
    const item = wrapper.findAll(".md__item").find((b) => b.text().includes("分离窗口"))!;
    expect(item.attributes("disabled")).toBeUndefined();
    await item.trigger("click");
    await flushPromises();
    expect(search.query).toBe("");
  });

  it("clears the search box after detaching from the plugin route", async () => {
    const { wrapper, active, search, router } = await mountMenu("/plugin/demo/index");
    active.set({ id: "demo", name: "demo", code: "index", title: "示例插件" });
    search.query = "残留文字";
    await wrapper.vm.$nextTick();
    const item = wrapper.findAll(".md__item").find((b) => b.text().includes("分离窗口"))!;
    await item.trigger("click");
    await flushPromises();
    expect(search.query).toBe("");
    expect(active.plugin).toBeNull();
    expect(router.currentRoute.value.name).toBe("launcher");
  });
});
