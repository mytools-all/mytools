import { describe, it, expect, vi, afterEach } from "vitest";
import { tauriAvailable, safeInvoke } from "@/lib/tauri";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (_cmd: string, _args?: Record<string, unknown>) => "pong"),
}));

type InternalsWindow = { __TAURI_INTERNALS__?: unknown };

afterEach(() => {
  delete (window as unknown as InternalsWindow).__TAURI_INTERNALS__;
});

describe("tauriAvailable", () => {
  it("is false without internals", () => {
    expect(tauriAvailable()).toBe(false);
  });

  it("is true when internals are present", () => {
    (window as unknown as InternalsWindow).__TAURI_INTERNALS__ = {};
    expect(tauriAvailable()).toBe(true);
  });
});

describe("safeInvoke", () => {
  it("returns null without internals", async () => {
    const out = await safeInvoke<string>("ping");
    expect(out).toBeNull();
  });

  it("forwards to invoke when tauri is available", async () => {
    (window as unknown as InternalsWindow).__TAURI_INTERNALS__ = {};
    const { invoke } = await import("@tauri-apps/api/core");
    const fn = invoke as unknown as ReturnType<typeof vi.fn>;
    fn.mockClear();
    const out = await safeInvoke<string>("ping", { a: 1 });
    expect(out).toBe("pong");
    expect(fn).toHaveBeenCalledWith("ping", { a: 1 });
  });
});