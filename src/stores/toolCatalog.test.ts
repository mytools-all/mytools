import type { LoadedPlugin } from "@/stores/plugins";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useToolCatalogStore } from "@/stores/toolCatalog";
import { TOOL_CATALOG } from "@/data/toolCatalog";
import snapshot from "@/data/catalogSnapshot.json";

const openMock = vi.fn<(args?: unknown) => Promise<string | null>>(async () => null);
const invokeMock = vi.fn<(cmd: string, args?: unknown) => Promise<unknown>>(
  async () => null,
);

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: (...args: unknown[]) => openMock(...(args as [])),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...(args as [string, unknown])),
}));

vi.mock("@/lib/tauri", () => ({
  tauriAvailable: () => true,
  safeInvoke: (...args: unknown[]) => invokeMock(...(args as [string, unknown])),
}));

describe("tool catalog store", () => {
  beforeEach(() => {
    localStorage.clear();
    setActivePinia(createPinia());
    openMock.mockReset();
    openMock.mockResolvedValue(null);
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(null);
  });

  it("exposes all indexed plugins with install metadata", () => {
    const s = useToolCatalogStore();
    expect(s.entries.length).toBeGreaterThanOrEqual(11);
    for (const entry of s.entries) {
      expect(entry.title.length).toBeGreaterThan(0);
      expect(entry.npm).toMatch(/^@mytools-all\//);
      expect(entry.category).toBeTruthy();
    }
  });

  it("marks tools that are ready for installation", () => {
    const s = useToolCatalogStore();
    const ready = s.entries.filter((e) => e.status === "ready");
    expect(ready.length).toBeGreaterThan(0);
    for (const entry of ready) {
      expect(s.stateOf(entry)).toBe("installable");
      expect(entry.npmSpec).toBeTruthy();
    }
  });

  it("installs the independent json tool", () => {
    const s = useToolCatalogStore();
    const json = s.entries.find((e) => e.name === "mytools-json")!;
    expect(json.status).toBe("ready");
    expect(s.stateOf(json)).toBe("installable");
  });

  it("marks unfinished tools as planned", () => {
    const s = useToolCatalogStore();
    const planned = [{ ...TOOL_CATALOG[0], status: "planned" as const }];
    for (const entry of planned) {
      expect(s.stateOf(entry)).toBe("planned");
    }
  });

  it("filters by category and query", () => {
    const s = useToolCatalogStore();
    s.category = "文件";
    expect(s.visibleEntries.every((e) => e.category === "文件")).toBe(true);
    s.category = "all";
    s.query = "markdown";
    expect(s.visibleEntries.map((e) => e.name)).toContain("mytools-markdown");
    s.query = "";
    expect(s.visibleEntries.length).toBe(s.entries.length);
  });

  it("installs a ready tool from its scoped npm package", async () => {
    const s = useToolCatalogStore();
    const entry = TOOL_CATALOG.find((e) => e.status === "ready")!;
    openMock.mockResolvedValue("C:\\tools\\demo");
    invokeMock.mockResolvedValue("mytools-demo");

    const name = await s.install(entry);

    expect(name).toBe("mytools-demo");
    expect(invokeMock).toHaveBeenCalledWith("plugin_install_npm", { spec: entry.npmSpec });
    expect(openMock).not.toHaveBeenCalled();
    expect(s.notice).toContain(entry.title);
  });

  it("refuses to install a planned tool", async () => {
    const s = useToolCatalogStore();
    const entry = { ...TOOL_CATALOG[0], status: "planned" as const };
    const name = await s.install(entry);
    expect(name).toBeNull();
    expect(s.error).toContain("开发中");
    expect(openMock).not.toHaveBeenCalled();
  });

  it("reports an error when the install fails", async () => {
    const s = useToolCatalogStore();
    const entry = TOOL_CATALOG.find((e) => e.status === "ready")!;
    openMock.mockResolvedValue("C:\\tools\\demo");
    invokeMock.mockRejectedValue(new Error("bad package"));

    const name = await s.install(entry);

    expect(name).toBeNull();
    expect(s.error).toContain("bad package");
  });

  it("loads remote metadata, categories and aliases then restores its cache", async () => {
    const data = structuredClone(snapshot);
    data.plugins[0].title = "远程 Demo";
    data.plugins[0].category = "远程分类";
    invokeMock.mockResolvedValue(data);
    const store = useToolCatalogStore();
    await store.refresh(true);
    expect(store.source).toBe("remote");
    expect(store.categories).toContain("远程分类");
    store.query = "mytools-demo";
    expect(store.visibleEntries[0].title).toBe("远程 Demo");
    setActivePinia(createPinia());
    const restored = useToolCatalogStore();
    expect(restored.source).toBe("cache");
    expect(restored.entries[0].title).toBe("远程 Demo");
  });

  it("keeps valid entries when remote JSON is invalid or unavailable", async () => {
    const store = useToolCatalogStore();
    const names = store.entries.map(entry => entry.name);
    invokeMock.mockResolvedValue({schemaVersion: 2, plugins: [{name: "broken"}]});
    await store.refresh(true);
    expect(store.entries.map(entry => entry.name)).toEqual(names);
    expect(store.loadError).toContain("继续使用已有目录");
    invokeMock.mockRejectedValue(new Error("offline"));
    await store.refresh(true);
    expect(store.entries.map(entry => entry.name)).toEqual(names);
  });

  it("falls back to a valid mirror when the primary returns invalid data", async () => {
    const { useCatalogSourcesStore } = await import("@/stores/catalogSources");
    const sources = useCatalogSourcesStore();
    invokeMock.mockImplementation(async (command, args) => {
      if (command === "plugin_catalog_sources_get") return { pluginList: sources.pluginList, pluginListMirror: sources.pluginListMirror };
      return (args as {url:string}).url === sources.pluginList ? { schemaVersion: 1 } : snapshot;
    });
    const store = useToolCatalogStore();
    await store.refresh(true);
    expect(store.source).toBe("remote");
    expect(sources.activeUrl).toBe(sources.pluginListMirror);
    expect(store.loadError).toBeNull();
  });

  it("uses the mirror after a primary network failure", async () => {
    const { useCatalogSourcesStore } = await import("@/stores/catalogSources");
    const sources = useCatalogSourcesStore();
    invokeMock.mockImplementation(async (command, args) => {
      if (command === "plugin_catalog_sources_get") return { pluginList: sources.pluginList, pluginListMirror: sources.pluginListMirror };
      if ((args as {url:string}).url === sources.pluginList) throw Error("timeout");
      return snapshot;
    });
    await useToolCatalogStore().refresh(true);
    expect(sources.activeUrl).toBe(sources.pluginListMirror);
    expect(useToolCatalogStore().loadError).toBeNull();
  });

  it("does not open a local directory picker for an incomplete store entry", async () => {
    const store = useToolCatalogStore();
    await store.install({...TOOL_CATALOG.find(entry => entry.status === "ready")!, npmSpec: undefined});
    expect(openMock).not.toHaveBeenCalled();
    expect(invokeMock).not.toHaveBeenCalled();
    expect(store.error).toContain("npm");
  });
});

describe("catalog plugin updates",()=>{
 beforeEach(()=>{localStorage.clear();setActivePinia(createPinia());invokeMock.mockReset();});
 it("compares installed versions and updates with an exact package and expected old version",async()=>{
  const {usePluginsStore}=await import("@/stores/plugins");const plugins=usePluginsStore();const catalog=useToolCatalogStore();const entry={...TOOL_CATALOG.find(e=>e.status==="ready")!,version:"0.2.0",npmSpec:"@mytools-all/mytools-search@0.2.0"};
  const plugin={manifest:{name:entry.name,version:"0.1.0"},enabled:false} as LoadedPlugin;plugins.plugins=[plugin];catalog.entries=[entry];
  expect(catalog.stateOf(entry)).toBe("updateable");
  invokeMock.mockImplementation(async command=>command==="plugin_list"?[{...plugin,manifest:{...plugin.manifest,version:"0.2.0"}}]:entry.name);
  await catalog.install(entry);expect(invokeMock).toHaveBeenCalledWith("plugin_update_npm",{spec:entry.npmSpec,code:entry.name,fromVersion:"0.1.0",version:"0.2.0"});expect(catalog.updates).toHaveLength(0);expect(plugins.plugins[0]?.enabled).toBe(false);
 });
 it("never offers equal, older or unpublished versions; failed upgrades retain the installed plugin",async()=>{
  const {usePluginsStore}=await import("@/stores/plugins");const plugins=usePluginsStore();const catalog=useToolCatalogStore();const entry=TOOL_CATALOG.find(e=>e.status==="ready")!;plugins.plugins=[{manifest:{name:entry.name,version:entry.version}} as LoadedPlugin];
  expect(catalog.canUpdate(entry)).toBe(false);expect(catalog.canUpdate({...entry,version:"0.0.0"})).toBe(false);expect(catalog.canUpdate({...entry,status:"planned",version:"9.0.0"})).toBe(false);
  const newer={...entry,version:"9.0.0",npmSpec:entry.npm+"@9.0.0"};invokeMock.mockRejectedValue(Error("download failed"));expect(await catalog.install(newer)).toBeNull();expect(plugins.plugins[0]?.manifest.version).toBe(entry.version);expect(invokeMock).not.toHaveBeenCalledWith("plugin_uninstall",expect.anything());
 });
});

it("batch updates continue after one failure and report the result",async()=>{
 setActivePinia(createPinia());localStorage.clear();invokeMock.mockReset();
 const {usePluginsStore}=await import("@/stores/plugins");const plugins=usePluginsStore(),catalog=useToolCatalogStore();
 const entries=TOOL_CATALOG.filter(e=>e.status==="ready").slice(0,2).map(e=>({...e,version:"0.2.0",npmSpec:e.npm+"@0.2.0"}));
 let installed=entries.map(e=>({manifest:{name:e.name,version:"0.1.0"}} as LoadedPlugin));plugins.plugins=installed;catalog.entries=entries;
 invokeMock.mockImplementation(async(command,args)=>{if(command==="plugin_list")return installed;const code=(args as {code:string}).code;if(code===entries[0]!.name)throw Error("network failure");installed=installed.map(p=>p.manifest.name===code?{...p,manifest:{...p.manifest,version:"0.2.0"}}:p);return code;});
 await catalog.updateAll();expect(catalog.notice).toContain("已更新 1 个插件");expect(catalog.notice).toContain("失败 1 个");expect(catalog.updatingAll).toBe(false);expect(catalog.updates).toHaveLength(1);
});
