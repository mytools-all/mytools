import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useGrantsStore } from "@/stores/grants";

const invokeMock = vi.fn(
  async (..._args: unknown[]): Promise<unknown> => null,
);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

function mountTauri() {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" } },
    invoke: invokeMock,
  };
}

describe("grants store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invokeMock.mockReset();
    invokeMock.mockImplementation(async () => null);
    mountTauri();
  });

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it("loads the stored grants", async () => {
    invokeMock.mockImplementation(async (...callArgs: unknown[]) => {
      const cmd = String(callArgs[0]);
      if (cmd === "plugin_capability_grants") {
        return [
          { plugin: "demo", capability: "fs.read", grantedAt: "2026-10-27T00:00:00Z" },
        ];
      }
      return null;
    });
    const store = useGrantsStore();
    await store.load();
    expect(store.grants).toHaveLength(1);
    expect(store.grants[0].plugin).toBe("demo");
    expect(store.error).toBeNull();
  });

  it("stays empty outside the desktop app", async () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    const store = useGrantsStore();
    await store.load();
    expect(store.grants).toEqual([]);
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("revokes a grant and reloads the list", async () => {
    const revoked: unknown[] = [];
    invokeMock.mockImplementation(async (...callArgs: unknown[]) => {
      const cmd = String(callArgs[0]);
      const args = callArgs[1];
      if (cmd === "plugin_revoke_capability") {
        revoked.push(args);
        return true;
      }
      if (cmd === "plugin_capability_grants") return [];
      return null;
    });
    const store = useGrantsStore();
    await store.revoke("demo", "fs.read");
    expect(revoked).toEqual([{ plugin: "demo", capability: "fs.read" }]);
    expect(store.grants).toEqual([]);
  });

  it("records a backend failure", async () => {
    invokeMock.mockImplementation(async (...callArgs: unknown[]) => {
      const cmd = String(callArgs[0]);
      if (cmd === "plugin_capability_grants") throw new Error("store unavailable");
      return null;
    });
    const store = useGrantsStore();
    await store.load();
    expect(store.error).toContain("store unavailable");
  });
});
