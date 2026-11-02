import { describe, it, expect, vi, beforeEach } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useClipboardStore } from "@/stores/clipboard";

const entries = [
  { id: 1, content: "alpha", pinned: false, created_at: new Date().toISOString() },
  { id: 2, content: "beta", pinned: true, created_at: new Date().toISOString() },
];

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === "clipboard_list") return entries;
    if (cmd === "clipboard_clear") return null;
    if (cmd === "clipboard_delete") return null;
    if (cmd === "clipboard_pin") return null;
    if (cmd === "clipboard_write") return null;
    return null;
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};

describe("clipboard store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("loads entries from the backend", async () => {
    const s = useClipboardStore();
    await s.refresh();
    expect(s.entries.length).toBe(2);
    expect(s.hasEntries).toBe(true);
    expect(s.selected?.id).toBe(1);
  });

  it("moves the cursor through the list", async () => {
    const s = useClipboardStore();
    await s.refresh();
    s.move(1);
    expect(s.selectedIndex).toBe(1);
    s.move(-1);
    expect(s.selectedIndex).toBe(0);
  });

  it("calls clipboard_clear on clearAll", async () => {
    const s = useClipboardStore();
    await s.refresh();
    const { invoke } = await import("@tauri-apps/api/core");
    const fn = invoke as unknown as ReturnType<typeof vi.fn>;
    fn.mockClear();
    await s.clearAll();
    expect(fn).toHaveBeenCalledWith("clipboard_clear");
  });

  it("writes text to the system clipboard via invoke", async () => {
    const s = useClipboardStore();
    const { invoke } = await import("@tauri-apps/api/core");
    const fn = invoke as unknown as ReturnType<typeof vi.fn>;
    fn.mockClear();
    await s.copyToClipboard("hello");
    expect(fn).toHaveBeenCalledWith("clipboard_write", { text: "hello" });
  });
});