import { describe, it, expect } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useActivePluginStore } from "@/stores/activePlugin";

describe("activePlugin store", () => {
  it("starts with no active plugin", () => {
    setActivePinia(createPinia());
    const s = useActivePluginStore();
    expect(s.plugin).toBeNull();
    expect(s.isActive).toBe(false);
  });

  it("set() stores the plugin and flips isActive", () => {
    setActivePinia(createPinia());
    const s = useActivePluginStore();
    s.set({ id: "demo", name: "demo", code: "index", title: "示例", icon: "i" });
    expect(s.plugin?.title).toBe("示例");
    expect(s.plugin?.code).toBe("index");
    expect(s.isActive).toBe(true);
  });

  it("clear() resets the plugin", () => {
    setActivePinia(createPinia());
    const s = useActivePluginStore();
    s.set({ id: "demo", name: "demo", code: "index", title: "示例" });
    s.clear();
    expect(s.plugin).toBeNull();
    expect(s.isActive).toBe(false);
  });
});