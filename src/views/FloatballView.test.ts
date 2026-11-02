import { mount } from "@vue/test-utils";
import { describe, it, expect, beforeEach, vi } from "vitest";
import FloatballView from "@/views/FloatballView.vue";

const invokeMock = vi.fn(
  async (_cmd: string, _args?: unknown): Promise<unknown> => null,
);

type PositionMock = { x: number; y: number };

const windowMock: Record<string, unknown> = {
  outerPosition: vi.fn(),
  setPosition: vi.fn(),
};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...(args as [string, unknown])),
}));

vi.mock("@tauri-apps/api/dpi", () => ({
  PhysicalPosition: class {
    readonly x: number;
    readonly y: number;
    constructor(x: number, y: number) {
      this.x = x;
      this.y = y;
    }
  },
}));

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => windowMock,
}));

describe("FloatballView", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(null);
    (windowMock.outerPosition as ReturnType<typeof vi.fn>).mockReset();
    (windowMock.setPosition as ReturnType<typeof vi.fn>).mockReset();
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  });

  it("renders the ball glyph", () => {
    const wrapper = mount(FloatballView);
    expect(wrapper.find(".floatball__ball").exists()).toBe(true);
    wrapper.unmount();
  });

  it("does nothing on click without drag when not in Tauri", async () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    const wrapper = mount(FloatballView);
    await wrapper.trigger("pointerdown", { screenX: 10, screenY: 10 });
    await wrapper.trigger("pointerup", { clientX: 10, clientY: 10 });
    expect(invokeMock).not.toHaveBeenCalled();
    expect(windowMock.outerPosition).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it("toggles the launcher on a click without drag", async () => {
    (windowMock.outerPosition as ReturnType<typeof vi.fn>).mockResolvedValue(
      { x: 120, y: 80 } as PositionMock,
    );
    const wrapper = mount(FloatballView);
    await wrapper.trigger("pointerdown", { screenX: 20, screenY: 20 });
    await wrapper.trigger("pointerup", { clientX: 20, clientY: 20 });
    await Promise.resolve();
    expect(invokeMock).toHaveBeenCalledWith("floatball_toggle_main");
    expect(invokeMock).not.toHaveBeenCalledWith(
      "floatball_save_position",
      expect.anything(),
    );
    wrapper.unmount();
  });

  it("moves the window while dragging and persists position on release", async () => {
    (windowMock.outerPosition as ReturnType<typeof vi.fn>).mockResolvedValue(
      { x: 120, y: 80 } as PositionMock,
    );
    const wrapper = mount(FloatballView);
    await wrapper.trigger("pointerdown", { screenX: 20, screenY: 20 });
    await wrapper.trigger("pointermove", { screenX: 60, screenY: 40 });
    await Promise.resolve();
    expect(windowMock.setPosition).toHaveBeenCalledWith(
      expect.objectContaining({ x: 160, y: 100 }),
    );
    await wrapper.trigger("pointerup", { clientX: 60, clientY: 40 });
    await Promise.resolve();
    expect(invokeMock).toHaveBeenCalledWith("floatball_save_position", {
      x: 120,
      y: 80,
    });
    expect(invokeMock).not.toHaveBeenCalledWith("floatball_toggle_main");
    wrapper.unmount();
  });

  it("does not treat a small jitter as a drag", async () => {
    (windowMock.outerPosition as ReturnType<typeof vi.fn>).mockResolvedValue(
      { x: 120, y: 80 } as PositionMock,
    );
    const wrapper = mount(FloatballView);
    await wrapper.trigger("pointerdown", { screenX: 20, screenY: 20 });
    await wrapper.trigger("pointermove", { screenX: 22, screenY: 21 });
    await Promise.resolve();
    expect(windowMock.setPosition).not.toHaveBeenCalled();
    await wrapper.trigger("pointerup", { clientX: 22, clientY: 21 });
    await Promise.resolve();
    expect(invokeMock).toHaveBeenCalledWith("floatball_toggle_main");
    wrapper.unmount();
  });

  it("releases drag when the pointer leaves the window", async () => {
    (windowMock.outerPosition as ReturnType<typeof vi.fn>).mockResolvedValue(
      { x: 120, y: 80 } as PositionMock,
    );
    const wrapper = mount(FloatballView);
    await wrapper.trigger("pointerdown", { screenX: 20, screenY: 20 });
    await wrapper.trigger("pointermove", { screenX: 100, screenY: 100 });
    await Promise.resolve();
    await wrapper.trigger("pointerleave", { clientX: 100, clientY: 100 });
    await Promise.resolve();
    expect(invokeMock).toHaveBeenCalledWith("floatball_save_position", {
      x: 120,
      y: 80,
    });
    wrapper.unmount();
  });
});