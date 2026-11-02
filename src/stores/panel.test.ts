import { describe, it, expect } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick } from "vue";
import { usePanelStore } from "./panel";
import type { LoadedPlugin } from "./plugins";

describe("panel plugin references", () => {
  it("persists a plugin feature once and deduplicates by identity instead of title", async () => {
    localStorage.clear();
    setActivePinia(createPinia());
    const store = usePanelStore();
    const plugin = {manifest:{name:"fixture",pluginName:"工具",features:[{code:"main"}]}} as LoadedPlugin;
    store.addPlugin(plugin);
    store.addPlugin({...plugin,manifest:{...plugin.manifest,pluginName:"新名称"}});
    await nextTick();
    expect(store.entries).toHaveLength(1);
    expect(store.hasPlugin("fixture","main")).toBe(true);
    expect(store.hasPlugin("fixture","other")).toBe(false);
    expect(JSON.parse(localStorage.getItem("mytools-panel-entries-v1") || "[]")[0]).toMatchObject({kind:"plugin",target:"fixture",feature:"main"});
  });
});
