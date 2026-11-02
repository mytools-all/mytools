import { flushPromises } from "@vue/test-utils";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useSearchStore } from "@/stores/search";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (_cmd: string, args: { query: string }) => {
    if (args.query === "=1+2") {
      return [{ title: "3", subtitle: "= 1+2", kind: "calculator", id: "x", score: 1 }];
    }
    if (args.query === "chrome") {
      return [
        { title: "Google Chrome", subtitle: "app (user)", kind: "app", id: "C:/chrome.lnk", score: 99 },
        { title: "chrome (built-in)", subtitle: "type: app", kind: "json-formatter", id: "json-formatter", score: 80 },
      ];
    }
    return [];
  }),
}));

(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};

describe("search store — advanced flows", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useFakeTimers();
  });

  it("surfaces calculator hits at the top regardless of other results", async () => {
    const s = useSearchStore();
    s.query = "=1+2";
    vi.advanceTimersByTime(60);
    await vi.runOnlyPendingTimersAsync();
    expect(s.results[0].kind).toBe("calculator");
  });

  it("keeps app launches and plugin navigation distinguishable by kind", async () => {
    const s = useSearchStore();
    s.query = "chrome";
    vi.advanceTimersByTime(60);
    await vi.runOnlyPendingTimersAsync();
    const kinds = s.results.map((r) => r.kind);
    expect(kinds).toContain("app");
  });

  it("selected wraps to the start when pressing ArrowUp", async () => {
    const s = useSearchStore();
    s.query = "chrome";
    vi.advanceTimersByTime(60);
    await vi.runOnlyPendingTimersAsync();
    expect(s.results.length).toBeGreaterThan(1);
    s.move(-1);
    expect(s.selectedIndex).toBe(s.results.length - 1);
    s.move(1);
    expect(s.selectedIndex).toBe(0);
  });
});

describe("search store — debounce", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useFakeTimers();
  });

  it("coalesces rapid keystrokes into one invoke", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const fn = invoke as unknown as ReturnType<typeof vi.fn>;
    fn.mockClear();

    const s = useSearchStore();
    s.query = "c";
    s.query = "ch";
    s.query = "chr";
    s.query = "chro";
    s.query = "chrom";
    s.query = "chrome";

    // Advance past debounce window.
    vi.advanceTimersByTime(60);
    await vi.runOnlyPendingTimersAsync();

    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith("search", { query: "chrome" });
  });
});

describe("search store — race protection", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useFakeTimers();
  });

  it("discards stale results when a newer query arrives", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const fn = invoke as unknown as ReturnType<typeof vi.fn>;
    let resolver: ((value: unknown) => void) | null = null;
    fn.mockImplementationOnce(
      () => new Promise((res) => {
        resolver = res;
      }),
    );

    const s = useSearchStore();
    s.query = "first";
    vi.advanceTimersByTime(60);
    await vi.runOnlyPendingTimersAsync();

    // Issue a second query while the first is still pending.
    s.query = "second";
    vi.advanceTimersByTime(60);
    await vi.runOnlyPendingTimersAsync();

    // Resolve the first query late — it must not overwrite results.
    resolver!([{ title: "stale", kind: "x", id: "x", subtitle: "", score: 1 }]);
    await flushPromises();
    expect(s.results.find((r) => r.title === "stale")).toBeUndefined();
  });
});