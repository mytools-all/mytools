import { mount } from "@vue/test-utils";
import { describe, it, expect, beforeEach, vi } from "vitest";
import WindowResizeHandles from "@/components/WindowResizeHandles.vue";

const { startResizeDraggingMock } = vi.hoisted(() => {
  const startResizeDraggingMock = vi.fn(async (_dir: string) => {});
  return { startResizeDraggingMock };
});

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ startResizeDragging: startResizeDraggingMock }),
}));

describe("WindowResizeHandles.vue", () => {
  beforeEach(() => {
    startResizeDraggingMock.mockClear();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it("renders all eight directional handles", () => {
    const wrapper = mount(WindowResizeHandles);
    const handles = wrapper.findAll(".resize-handle");
    expect(handles.length).toBe(8);
    for (const dir of ["n", "s", "e", "w", "ne", "nw", "se", "sw"]) {
      expect(wrapper.find(`.resize-handle--${dir}`).exists()).toBe(true);
    }
  });

  it("applies the correct resize cursors", () => {
    const wrapper = mount(WindowResizeHandles);
    const cursors = [
      ["n", "ns-resize"],
      ["s", "ns-resize"],
      ["e", "ew-resize"],
      ["w", "ew-resize"],
      ["ne", "nesw-resize"],
      ["nw", "nwse-resize"],
      ["se", "nwse-resize"],
      ["sw", "nesw-resize"],
    ] as const;
    for (const [dir, cursor] of cursors) {
      expect(wrapper.find(`.resize-handle--${dir}`).attributes("style")).toContain(
        cursor,
      );
    }
  });

  it("calls startResizeDragging with the handle direction", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" } },
      transformCallback: () => 0,
      invoke: vi.fn(async () => "pong"),
    };
    const wrapper = mount(WindowResizeHandles);
    const se = wrapper.find(".resize-handle--se");
    await se.trigger("pointerdown", { pointerId: 1 });
    expect(startResizeDraggingMock).toHaveBeenCalledWith("se");
  });

  it("does not call startResizeDragging outside Tauri", async () => {
    const wrapper = mount(WindowResizeHandles);
    const w = wrapper.find(".resize-handle--e");
    await w.trigger("pointerdown", { pointerId: 1 });
    expect(startResizeDraggingMock).not.toHaveBeenCalled();
  });
});