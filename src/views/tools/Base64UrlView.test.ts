import { mount, flushPromises } from "@vue/test-utils";
import { describe, it, expect } from "vitest";
import Base64UrlView from "@/views/tools/Base64UrlView.vue";

describe("Base64UrlView", () => {
  it("encodes UTF-8 text to Base64 round-trip", async () => {
    const wrapper = mount(Base64UrlView, { attachTo: document.body });
    const vm = wrapper.vm as unknown as {
      input: string;
      mode: string;
      output: string;
      tryRun: () => void;
    };

    vm.mode = "encode-base64";
    vm.input = "Hello, 世界";
    await flushPromises();
    vm.tryRun();
    await flushPromises();
    expect(vm.output).toBeTruthy();
    const encoded = vm.output;

    vm.mode = "decode-base64";
    vm.input = encoded;
    await flushPromises();
    vm.tryRun();
    await flushPromises();
    expect(vm.output).toBe("Hello, 世界");
    wrapper.unmount();
  });

  it("URL encodes then decodes", async () => {
    const wrapper = mount(Base64UrlView, { attachTo: document.body });
    const vm = wrapper.vm as unknown as {
      input: string;
      mode: string;
      output: string;
      tryRun: () => void;
    };

    vm.mode = "encode-url";
    vm.input = "https://example.com/?q=中文";
    await flushPromises();
    vm.tryRun();
    await flushPromises();
    expect(vm.output).toContain("%E4%B8%AD");
    const encoded = vm.output;

    vm.mode = "decode-url";
    vm.input = encoded;
    await flushPromises();
    vm.tryRun();
    await flushPromises();
    expect(vm.output).toBe("https://example.com/?q=中文");
    wrapper.unmount();
  });
});