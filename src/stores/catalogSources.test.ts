import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useCatalogSourcesStore } from "./catalogSources";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@/lib/tauri", () => ({ tauriAvailable: () => true }));
describe("catalog sources", () => {
  beforeEach(() => { setActivePinia(createPinia()); invoke.mockReset(); });
  it("loads saved sources and persists trimmed edits", async () => {
    invoke.mockResolvedValueOnce({ pluginList: "https://example.com/a.json", pluginListMirror: "https://example.com/b.json" });
    const store = useCatalogSourcesStore();
    await store.load();
    expect(store.pluginList).toBe("https://example.com/a.json");
    invoke.mockResolvedValueOnce(null);
    await store.save(" https://example.com/c.json ", store.pluginListMirror);
    expect(invoke).toHaveBeenLastCalledWith("plugin_catalog_sources_set", { sources: { pluginList: "https://example.com/c.json", pluginListMirror: "https://example.com/b.json" } });
    expect(store.pluginList).toBe("https://example.com/c.json");
  });
  it("rejects insecure or credential-bearing addresses before persistence", async () => {
    const store = useCatalogSourcesStore();
    const previous = store.pluginList;
    await expect(store.save("http://example.com/a", store.pluginListMirror)).rejects.toThrow();
    await expect(store.save("https://user:secret@example.com/a", store.pluginListMirror)).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();
    expect(store.pluginList).toBe(previous);
  });
  it("keeps current settings if persistence fails", async () => {
    invoke.mockRejectedValueOnce(Error("disk unavailable"));
    const store = useCatalogSourcesStore();
    const previous = store.pluginList;
    await expect(store.save("https://example.com/new.json", store.pluginListMirror)).rejects.toThrow("disk unavailable");
    expect(store.pluginList).toBe(previous);
  });
});
