import { mount } from "@vue/test-utils";
import { describe, it, expect, beforeEach } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import TopTabNav from "@/components/TopTabNav.vue";

interface Tab {
  id: string;
  label: string;
  count?: number;
  tone?: "ok" | "warn" | "err";
  title?: string;
  disabled?: boolean;
}

describe("TopTabNav", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  function mountNav(tabs: Tab[], modelValue = "a") {
    const pinia = createPinia();
    setActivePinia(pinia);
    return mount(TopTabNav, {
      props: { tabs, modelValue },
      global: { plugins: [pinia] },
    });
  }

  it("renders one button per tab", () => {
    const wrapper = mountNav([
      { id: "a", label: "全部" },
      { id: "b", label: "开发" },
    ]);
    expect(wrapper.findAll(".ttn__tab").length).toBe(2);
    expect(wrapper.text()).toContain("全部");
    expect(wrapper.text()).toContain("开发");
  });

  it("marks the active tab", () => {
    const wrapper = mountNav([
      { id: "a", label: "全部" },
      { id: "b", label: "开发" },
    ], "b");
    const active = wrapper.find(".ttn__tab--active");
    expect(active.exists()).toBe(true);
    expect(active.text()).toContain("开发");
  });

  it("renders a count badge when count is present", () => {
    const wrapper = mountNav([{ id: "a", label: "全部", count: 42 }]);
    expect(wrapper.find(".ttn__count").text()).toBe("42");
  });

  it("omits the count badge when count is undefined", () => {
    const wrapper = mountNav([{ id: "a", label: "全部" }]);
    expect(wrapper.find(".ttn__count").exists()).toBe(false);
  });

  it("applies a tone class", () => {
    const wrapper = mountNav([{ id: "a", label: "启用", tone: "ok" }]);
    expect(wrapper.find(".ttn__tab--ok").exists()).toBe(true);
  });

  it("emits update:modelValue on click", async () => {
    const wrapper = mountNav([
      { id: "a", label: "全部" },
      { id: "b", label: "开发" },
    ]);
    await wrapper.findAll(".ttn__tab")[1].trigger("click");
    expect(wrapper.emitted("update:modelValue")![0]).toEqual(["b"]);
  });

  it("applies the title attribute when provided", () => {
    const wrapper = mountNav([{ id: "a", label: "全部", title: "提示" }]);
    expect(wrapper.find(".ttn__tab").attributes("title")).toBe("提示");
  });

  it("renders disabled tabs as disabled buttons", () => {
    const wrapper = mountNav([{ id: "a", label: "其它", disabled: true }]);
    expect(wrapper.find(".ttn__tab").attributes("disabled")).toBeDefined();
  });
});