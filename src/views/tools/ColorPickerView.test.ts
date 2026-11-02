import { mount, flushPromises } from "@vue/test-utils";
import { describe, it, expect, vi, beforeEach } from "vitest";
import ColorPickerView from "@/views/tools/ColorPickerView.vue";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (cmd: string, args: { input: string }) => invokeMock(cmd, args),
}));

describe("ColorPickerView", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("parses HEX input on demand and shows the swatch", async () => {
    invokeMock.mockImplementation(async (_cmd: string, args: { input: string }) => {
      if (args.input === "#6cf") {
        return { hex: "#66CCFF", rgb: "rgb(102, 204, 255)", r: 102, g: 204, b: 255 };
      }
      throw new Error("bad input");
    });
    const wrapper = mount(ColorPickerView, { attachTo: document.body });
    const inputs = wrapper.findAll("input.color__input");
    await inputs[0].setValue("#6cf");
    await flushPromises();
    await wrapper.findAll("button.color__btn")[0].trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("#66CCFF");
    expect(wrapper.text()).toContain("rgb(102, 204, 255)");
    expect(wrapper.find(".color__swatch").exists()).toBe(true);
    wrapper.unmount();
  });

  it("shows an error for invalid input", async () => {
    invokeMock.mockImplementation(async () => {
      throw new Error("bad input");
    });
    const wrapper = mount(ColorPickerView, { attachTo: document.body });
    const inputs = wrapper.findAll("input.color__input");
    await inputs[0].setValue("not-a-color");
    await flushPromises();
    await wrapper.findAll("button.color__btn")[0].trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("bad input");
    expect(wrapper.find(".color__swatch").exists()).toBe(false);
    wrapper.unmount();
  });
});