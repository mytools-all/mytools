import { beforeEach, expect, it, vi } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import InstalledPluginsPage from "./InstalledPluginsPage.vue";
import { useToolCatalogStore } from "@/stores/toolCatalog";
import { usePluginsStore, type LoadedPlugin } from "@/stores/plugins";
import { useCenterSearchStore } from "@/stores/centerSearch";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

beforeEach(() => { localStorage.clear(); setActivePinia(createPinia()); });

it("limits selected updates to checked rows and clears selection after search changes", async () => {
  const catalog = useToolCatalogStore();
  vi.spyOn(catalog, "initialize").mockResolvedValue();
  const entries = catalog.entries.filter(entry => entry.status === "ready").slice(0, 2).map(entry => ({ ...entry, version: "9.0.0", npmSpec: entry.npm + "@9.0.0" }));
  catalog.entries = entries;
  usePluginsStore().plugins = entries.map(entry => ({ manifest: { name: entry.name, pluginName: entry.title, version: "0.1.0", description: "test", features: [] }, enabled: true, origin: "official", declaredCapabilities: [] } as unknown as LoadedPlugin));
  const updateSelected = vi.spyOn(catalog, "updateSelected").mockResolvedValue();
  const updateAll = vi.spyOn(catalog, "updateAll").mockResolvedValue();
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: "/", component: { template: "<div/>" } }] });
  await router.push("/");
  const wrapper = mount(InstalledPluginsPage, { props: { detached: false }, global: { plugins: [router] } });
  await wrapper.findAll("button").find(button => button.text().includes("可更新"))!.trigger("click");
  await flushPromises();
  const boxes = wrapper.findAll<HTMLInputElement>('.pc__iitem input[type="checkbox"]');
  expect(boxes).toHaveLength(2);
  await boxes[0]!.setValue(true);
  await wrapper.findAll("button").find(button => button.text().startsWith("更新已选"))!.trigger("click");
  expect(updateSelected).toHaveBeenCalledWith([entries[0]!.name]);
  useCenterSearchStore().query = entries[1]!.name;
  await flushPromises();
  expect(wrapper.findAll("button").find(button => button.text().startsWith("更新已选"))!.attributes("disabled")).toBeDefined();
  await wrapper.findAll("button").find(button => button.text().startsWith("更新全部"))!.trigger("click");
  expect(updateAll).toHaveBeenCalledOnce();
  wrapper.unmount();
});
