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

  it("does not open a local directory picker for an incomplete store entry", async () => {
    const store = useToolCatalogStore();
    await store.install({...TOOL_CATALOG.find(entry => entry.status === "ready")!, npmSpec: undefined});
    expect(openMock).not.toHaveBeenCalled();
    expect(invokeMock).not.toHaveBeenCalled();
    expect(store.error).toContain("npm");
  });
});
