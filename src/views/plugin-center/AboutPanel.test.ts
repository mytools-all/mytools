import { createPinia, setActivePinia } from "pinia";
import { mount, flushPromises } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { openUrl } from "@tauri-apps/plugin-opener";
import { APP_INFO } from "@/config/app";
import AboutPanel from "./AboutPanel.vue";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => null) }));
vi.mock("@tauri-apps/plugin-opener", () => ({openUrl:vi.fn(async () => {})}));
vi.mock("@/lib/tauri", () => ({tauriAvailable:() => true}));

describe("AboutPanel", () => {
  beforeEach(() => { setActivePinia(createPinia()); vi.mocked(openUrl).mockReset(); });

  it("shows global metadata and opens the configured repository in the system browser", async () => {
    const wrapper = mount(AboutPanel);
    expect(wrapper.text()).toContain(APP_INFO.name);
    expect(wrapper.text()).toContain(`v${APP_INFO.version}`);
    expect(wrapper.text()).toContain(APP_INFO.author);
    expect(wrapper.text()).toContain(APP_INFO.license);
    const repository = wrapper.get(`a[href="${APP_INFO.repository}"]`);
    await repository.trigger("click");
    await flushPromises();
    expect(openUrl).toHaveBeenCalledWith(APP_INFO.repository);
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
    wrapper.unmount();
  });

  it("reports a failed browser launch instead of rejecting the click handler", async () => {
    vi.mocked(openUrl).mockRejectedValueOnce(new Error("系统浏览器不可用"));
    const wrapper = mount(AboutPanel);
    await wrapper.get(`a[href="${APP_INFO.issues}"]`).trigger("click");
    await flushPromises();
    expect(wrapper.get('[role="alert"]').text()).toContain("系统浏览器不可用");
    wrapper.unmount();
  });
});
