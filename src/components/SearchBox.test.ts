import { mount, flushPromises } from "@vue/test-utils";
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import SearchBox from "@/components/SearchBox.vue";
import { useSearchStore } from "@/stores/search";
import { usePluginSearchStore } from "@/stores/pluginSearch";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (_cmd: string, args?: { query?: string }) => {
    if (args?.query === "json") {
      return [
        {
          kind: "json-formatter",
          id: "json-formatter",
          title: "Fresh JSON",
          subtitle: "type: json-formatter",
          score: 100,
        },
      ];
    }
    return [];
  }),
}));

(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};

describe("SearchBox", () => {
  afterEach(() => { vi.clearAllTimers(); vi.useRealTimers(); });

  afterEach(() => { vi.clearAllTimers(); vi.useRealTimers(); });

  beforeEach(() => {
    vi.useFakeTimers();
    vi.useFakeTimers();
    setActivePinia(createPinia());
  });

  async function mountBox(path = "/") {
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
    const search = useSearchStore();
    const wrapper = mount(SearchBox, { global: { plugins: [pinia, router] } });
    return { wrapper, search };
  }

  it("emits back on Backspace with empty query on the plugin route", async () => {
    const { wrapper } = await mountBox("/plugin/demo/index");
    await wrapper.find("input").trigger("keydown", { key: "Backspace" });
    expect(wrapper.emitted("back")).toBeTruthy();
  });
  it("routes opted-in plugin input to the plugin instead of the global query", async () => {
    const {wrapper,search}=await mountBox("/plugin/fixture/main");
    const plugin=usePluginSearchStore();plugin.bind("fixture",true,"搜索文件");
    await flushPromises();
    await wrapper.find("input").setValue("report.txt");
    expect(plugin.query).toBe("report.txt");
    expect(search.query).toBe("");
    await wrapper.find("input").trigger("keydown",{key:"Enter"});
    expect(plugin.key).toBe("Enter");
    expect(wrapper.emitted("select")).toBeUndefined();
    wrapper.unmount();
  });

  it("emits back on Backspace with empty query on /plugins", async () => {
    const { wrapper } = await mountBox("/plugins");
    await wrapper.find("input").trigger("keydown", { key: "Backspace" });
    expect(wrapper.emitted("back")).toBeTruthy();
  });

  it("does not emit back on the launcher route", async () => {
    const { wrapper } = await mountBox();
    await wrapper.find("input").trigger("keydown", { key: "Backspace" });
    expect(wrapper.emitted("back")).toBeFalsy();
  });

  it("does not emit back when the query is non-empty", async () => {
    const { wrapper, search } = await mountBox("/plugin/demo/index");
    search.query = "abc";
    await wrapper.find("input").trigger("keydown", { key: "Backspace" });
    expect(wrapper.emitted("back")).toBeFalsy();
  });

  it("Enter on a route command selects immediately and clears the query", async () => {
    const { wrapper, search } = await mountBox();
    search.query = "设置";
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    const emitted = wrapper.emitted("select");
    expect(emitted).toBeTruthy();
    expect(emitted![0][0]).toMatchObject({ kind: "route" });
    expect(search.query).toBe("");
  });

  it("Enter waits for fresh backend results instead of using the stale batch", async () => {
    const { wrapper, search } = await mountBox();
    search.results = [
      { kind: "tool", id: "old", title: "Stale Result", subtitle: "", score: 1 },
    ];
    search.selectedIndex = 0;
    search.query = "json";
    const done = wrapper.find("input").trigger("keydown", { key: "Enter" });
    await done;
    await flushPromises();
    const emitted = wrapper.emitted("select");
    expect(emitted).toBeTruthy();
    expect(emitted![0][0]).toMatchObject({ title: "Fresh JSON" });
  });

  it("ignores Enter while an IME composition is active", async () => {
    const { wrapper, search } = await mountBox();
    search.query = "设置";
    await wrapper
      .find("input")
      .trigger("keydown", { key: "Enter", isComposing: true });
    expect(wrapper.emitted("select")).toBeFalsy();
    expect(search.query).toBe("设置");
  });

  it("Enter with an empty query does nothing", async () => {
    const { wrapper } = await mountBox();
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("select")).toBeFalsy();
  });
});
