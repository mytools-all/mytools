import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAppIndexStore, type ListedApp } from "@/stores/appIndex";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(async (..._args: unknown[]): Promise<unknown> => null),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

const apps: ListedApp[] = [
  { name: "记事本", target: "Notepad.exe", subtitle: "app (start menu)" },
  { name: "设置", target: "windows.settings!App", subtitle: "app (system)" },
];

function mountTauri() {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" } },
    invoke: invokeMock,
  };
}

function replyWith(handler: (cmd: string) => unknown) {
  invokeMock.mockImplementation(async (...args: unknown[]) => handler(args[0] as string));
}

describe("appIndex store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invokeMock.mockReset();
    invokeMock.mockImplementation(async () => null);
    mountTauri();
  });

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it("marks itself loaded without calling Tauri when unavailable", async () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    const store = useAppIndexStore();
    await store.ensureLoaded();
    expect(store.loaded).toBe(true);
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("downloads the snapshot and indexes it case-insensitively", async () => {
    replyWith((cmd) => (cmd === "app_index_list" ? apps : null));
    const store = useAppIndexStore();
    await store.ensureLoaded();
    expect(store.entries).toHaveLength(2);
    expect(store.find("notepad.EXE")?.name).toBe("记事本");
    expect(store.find("WINDOWS.SETTINGS!APP")?.subtitle).toBe("app (system)");
    expect(store.find("missing.exe")).toBeUndefined();
  });

  it("reuses a single in-flight load", async () => {
    let resolveList: ((value: unknown) => void) | undefined;
    invokeMock.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveList = resolve;
        }),
    );
    const store = useAppIndexStore();
    const first = store.ensureLoaded();
    const second = store.ensureLoaded();
    await vi.waitFor(() => expect(resolveList).toBeTruthy());
    resolveList?.(apps);
    await Promise.all([first, second]);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(store.entries).toHaveLength(2);
  });

  it("reloads the snapshot after the backend reports a stale index", async () => {
    replyWith((cmd) => {
      if (cmd === "app_index_refresh_if_stale") return true;
      if (cmd === "app_index_list") return apps;
      return null;
    });
    const store = useAppIndexStore();
    await store.refreshIfStale();
    const calls = invokeMock.mock.calls.map((c) => c[0]);
    expect(calls).toEqual(["app_index_refresh_if_stale", "app_index_list"]);
    expect(store.entries).toHaveLength(2);
  });

  it("throttles rapid focus events into one refresh", async () => {
    replyWith((cmd) => (cmd === "app_index_refresh_if_stale" ? false : null));
    const store = useAppIndexStore();
    await store.refreshIfStale();
    await store.refreshIfStale();
    await store.refreshIfStale();
    const refreshCalls = invokeMock.mock.calls.filter(
      (c) => c[0] === "app_index_refresh_if_stale",
    );
    expect(refreshCalls).toHaveLength(1);
  });

  it("resolves and caches an icon per target", async () => {
    replyWith((cmd) =>
      cmd === "app_resolve"
        ? { name: "记事本", subtitle: "app (start menu)", icon: "data:image/png;base64,AAA" }
        : null,
    );
    const store = useAppIndexStore();
    await store.ensureIcon("Notepad.exe");
    await store.ensureIcon("notepad.exe");
    expect(store.iconFor("NOTEPAD.EXE")).toBe("data:image/png;base64,AAA");
    const resolveCalls = invokeMock.mock.calls.filter((c) => c[0] === "app_resolve");
    expect(resolveCalls).toHaveLength(1);
    expect(invokeMock.mock.calls.filter((c) => c[0] === "app_icon")).toHaveLength(0);
  });

  it("falls back to app_icon when the target is not in the index", async () => {
    replyWith((cmd) => {
      if (cmd === "app_resolve") return null;
      if (cmd === "app_icon") return "data:image/png;base64,FALLBACK";
      return null;
    });
    const store = useAppIndexStore();
    await store.ensureIcon("Notepad.exe");
    expect(store.iconFor("notepad.exe")).toBe("data:image/png;base64,FALLBACK");
    const iconCalls = invokeMock.mock.calls.filter((c) => c[0] === "app_icon");
    expect(iconCalls).toHaveLength(1);
    expect(iconCalls[0][1]).toEqual({ path: "Notepad.exe" });
  });

  it("falls back to app_icon when app_resolve throws", async () => {
    replyWith((cmd) => {
      if (cmd === "app_resolve") throw new Error("shell unavailable");
      if (cmd === "app_icon") return "data:image/png;base64,RECOVERED";
      return null;
    });
    const store = useAppIndexStore();
    await store.ensureIcon("ghost.exe");
    expect(store.iconFor("ghost.exe")).toBe("data:image/png;base64,RECOVERED");
  });

  it("caches a negative icon result after both lookups fail", async () => {
    replyWith((cmd) => {
      if (cmd === "app_resolve") throw new Error("shell unavailable");
      return null;
    });
    const store = useAppIndexStore();
    await store.ensureIcon("ghost.exe");
    await store.ensureIcon("ghost.exe");
    expect(store.iconFor("ghost.exe")).toBeUndefined();
    expect(invokeMock).toHaveBeenCalledTimes(2);
  });
});
