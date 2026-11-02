import { describe, it, expect, beforeEach, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useToolCatalogStore } from "@/stores/toolCatalog";
import { TOOL_CATALOG } from "@/data/toolCatalog";

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
    setActivePinia(createPinia());
    openMock.mockReset();
    openMock.mockResolvedValue(null);
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(null);
  });

  it("exposes every planned tool with a rubick reference", () => {
    const s = useToolCatalogStore();
    expect(s.entries.length).toBeGreaterThanOrEqual(11);
    for (const entry of s.entries) {
      expect(entry.title.length).toBeGreaterThan(0);
      expect(entry.reference?.package ?? "").toBeTruthy();
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
});
