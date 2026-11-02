import { mount, flushPromises } from "@vue/test-utils";
import { describe, it, expect } from "vitest";
import JsonFormatterView from "@/views/tools/JsonFormatterView.vue";

describe("JsonFormatterView", () => {
  it("formats a JSON input on demand", async () => {
    const wrapper = mount(JsonFormatterView, { attachTo: document.body });
    const textareas = wrapper.findAll("textarea");
    expect(textareas.length).toBe(2);

    const vm = wrapper.vm as unknown as { input: string; tryFormat: () => void };
    vm.input = '{"a":1,"b":[2,3]}';
    await flushPromises();
    vm.tryFormat();
    await flushPromises();

    expect((textareas[1].element as HTMLTextAreaElement).value).toContain('"a": 1');
    wrapper.unmount();
  });

  it("shows an error for invalid JSON", async () => {
    const wrapper = mount(JsonFormatterView, { attachTo: document.body });
    const vm = wrapper.vm as unknown as { input: string; tryFormat: () => void };
    vm.input = "{broken";
    await flushPromises();
    vm.tryFormat();
    await flushPromises();
    expect(wrapper.text()).toContain("JSON 解析失败");
    wrapper.unmount();
  });
});