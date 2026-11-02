import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { usePluginsStore } from "./plugins";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@/lib/tauri", () => ({ tauriAvailable: () => true }));

describe("plugin snapshot cache", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
  });

  it("shares the initial request and reuses it across page entries", async () => {
    let finish!: (value: unknown[]) => void;
    invoke.mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
    const store = usePluginsStore();
    const first = store.ensureLoaded();
    const second = store.ensureLoaded();
    expect(invoke).toHaveBeenCalledTimes(1);
    finish([]);
    await Promise.all([first, second]);
    await store.ensureLoaded();
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(store.loaded).toBe(true);
  });

  it("retains invalidation arriving during an in-flight request", async () => {
    let finish!: (value: unknown[]) => void;
    invoke.mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; }));
    invoke.mockResolvedValue([]);
    const store = usePluginsStore();
    const first = store.ensureLoaded();
    const changed = store.refresh();
    finish([]);
    await Promise.all([first, changed]);
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it("retries after a failed initial load", async () => {
    invoke.mockRejectedValueOnce(new Error("unavailable")).mockResolvedValue([]);
    const store = usePluginsStore();
    await store.ensureLoaded();
    expect(store.loaded).toBe(false);
    await store.ensureLoaded();
    expect(store.loaded).toBe(true);
    expect(store.error).toBeNull();
  });
});
