import { mount, flushPromises } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia } from "pinia";
import MyPanelView from "./MyPanelView.vue";

const {hide} = vi.hoisted(() => ({hide:vi.fn(async()=>{})}));
vi.mock("@/stores/plugins",()=>({usePluginsStore:()=>({plugins:[],enabledPlugins:[],ensureLoaded:async()=>{}})}));
vi.mock("@/stores/appIndex",()=>({useAppIndexStore:()=>({entries:[],ensureLoaded:async()=>{},iconFor:()=>null})}));
vi.mock("@/lib/tauri",()=>({tauriAvailable:()=>true,safeInvoke:vi.fn(async()=>true)}));
vi.mock("@tauri-apps/api/window",()=>({getCurrentWindow:()=>({hide})}));

describe("My Panel add entry",()=>{
  beforeEach(()=>{localStorage.clear();hide.mockClear();});

  async function setup() {
    const wrapper = mount(MyPanelView,{global:{plugins:[createPinia()]}});
    await flushPromises();
    await wrapper.get("[aria-controls='panel-add-form']").trigger("click");
    return wrapper;
  }

  it("closes the add form without hiding the panel",async()=>{
    const wrapper = await setup();
    expect(wrapper.find(".add-form").exists()).toBe(true);
    await wrapper.get(".add-form__close").trigger("click");
    expect(wrapper.find(".add-form").exists()).toBe(false);
    expect(hide).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it("uses Escape to leave adding before closing the panel",async()=>{
    const wrapper = await setup();
    document.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",cancelable:true}));
    await flushPromises();
    expect(wrapper.find(".add-form").exists()).toBe(false);
    expect(hide).not.toHaveBeenCalled();
    document.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",cancelable:true}));
    await flushPromises();
    expect(hide).toHaveBeenCalledOnce();
    wrapper.unmount();
  });
});
