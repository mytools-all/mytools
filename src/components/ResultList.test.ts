import { mount, flushPromises } from "@vue/test-utils";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import ResultList from "@/components/ResultList.vue";
import type { SearchResult } from "@/stores/search";

function makeItem(overrides: Partial<SearchResult> = {}): SearchResult {
  return {
    id: "demo-hello:index",
    kind: "plugin",
    title: "Demo Hello",
    subtitle: "hello world",
    score: 1,
    icon: "data:image/svg+xml;utf8,<svg/>",
    ...overrides,
  };
}

function mountList(items: SearchResult[] = [makeItem()]) {
  return mount(ResultList, {
    props: { results: items, selectedIndex: 0 },
  });
}

describe("ResultList ☆ 按钮（pin 交互）", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useRealTimers();
  });

  it("左键单击 ☆ 触发 pin 事件", async () => {
    const wrapper = mountList();
    const btn = wrapper.find(".result-list__pin");
    expect(btn.exists()).toBe(true);
    await btn.trigger("click");
    expect(wrapper.emitted("pin")).toBeTruthy();
    expect(wrapper.emitted("pin")![0]).toEqual([
      expect.objectContaining({ id: "demo-hello:index" }),
    ]);
  });

  it("右键 ☆ 触发 pin 事件（preventDefault 系统右键菜单）", async () => {
    const wrapper = mountList();
    const btn = wrapper.find(".result-list__pin");
    await btn.trigger("contextmenu");
    expect(wrapper.emitted("pin")).toBeTruthy();
    expect(wrapper.emitted("pin")![0]).toEqual([
      expect.objectContaining({ id: "demo-hello:index" }),
    ]);
  });

  it("长按 500ms 触发 pin 事件", async () => {
    vi.useFakeTimers();
    const wrapper = mountList();
    const btn = wrapper.find(".result-list__pin");
    await btn.trigger("pointerdown", { button: 0 });
    expect(wrapper.emitted("pin")).toBeFalsy();
    await vi.advanceTimersByTimeAsync(500);
    expect(wrapper.emitted("pin")).toBeTruthy();
  });

  it("长按中滑动（pointermove）取消，不触发 pin", async () => {
    vi.useFakeTimers();
    const wrapper = mountList();
    const btn = wrapper.find(".result-list__pin");
    await btn.trigger("pointerdown", { button: 0 });
    await vi.advanceTimersByTimeAsync(200);
    await btn.trigger("pointermove");
    await vi.advanceTimersByTimeAsync(500);
    expect(wrapper.emitted("pin")).toBeFalsy();
  });

  it("长按中 pointerup 提前释放，不触发 pin", async () => {
    vi.useFakeTimers();
    const wrapper = mountList();
    const btn = wrapper.find(".result-list__pin");
    await btn.trigger("pointerdown", { button: 0 });
    await vi.advanceTimersByTimeAsync(200);
    await btn.trigger("pointerup");
    await vi.advanceTimersByTimeAsync(500);
    expect(wrapper.emitted("pin")).toBeFalsy();
  });

  it("长按 500ms 期间按钮加 .is-pressing 视觉反馈", async () => {
    vi.useFakeTimers();
    const wrapper = mountList();
    const btn = wrapper.find(".result-list__pin");
    expect(btn.classes()).not.toContain("is-pressing");
    await btn.trigger("pointerdown", { button: 0 });
    await flushPromises();
    expect(btn.classes()).toContain("is-pressing");
  });
});