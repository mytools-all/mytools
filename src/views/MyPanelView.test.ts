import { mount, flushPromises } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia } from "pinia";
import {safeInvoke} from "@/lib/tauri";
import MyPanelView from "./MyPanelView.vue";

const {hide} = vi.hoisted(() => ({hide:vi.fn(async()=>{})}));
vi.mock("@/stores/plugins",()=>({usePluginsStore:()=>({plugins:[{manifest:{name:"mytools-ocr",pluginName:"图片 OCR",features:[{code:"ocr",explain:"图片识别"}]} }],enabledPlugins:[{manifest:{name:"mytools-ocr",pluginName:"图片 OCR",features:[{code:"ocr",explain:"图片识别"}]} }],ensureLoaded:async()=>{},refresh:async()=>{}})}));
vi.mock("@/stores/settings",()=>({useSettingsStore:()=>({init:async()=>{}})}));
vi.mock("@tauri-apps/api/event",()=>({listen:async()=>()=>{}}));
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

it("shows updated plugin names and routes successful capture into OCR",async()=>{
 localStorage.clear();
 localStorage.setItem("mytools-panel-entries-v1",JSON.stringify([{id:"ocr",kind:"plugin",target:"mytools-ocr",feature:"ocr",title:"讯飞 OCR"}]));
 const wrapper=mount(MyPanelView,{global:{plugins:[createPinia()]}});await flushPromises();
 expect(wrapper.get(".entry").text()).toContain("图片 OCR");
 const button=wrapper.findAll(".capture-tools button")[1];expect(button).toBeTruthy();await button!.trigger("click");await flushPromises();
 expect(safeInvoke).toHaveBeenCalledWith("panel_capture_image");expect(safeInvoke).toHaveBeenCalledWith("plugin_open_window",{pluginId:"mytools-ocr",feature:"ocr",fromPanel:true});wrapper.unmount();
});

it('does not launch OCR when screenshot selection is cancelled',async()=>{
 localStorage.clear();
 const wrapper=mount(MyPanelView,{global:{plugins:[createPinia()]}});await flushPromises();
 vi.mocked(safeInvoke).mockClear();vi.mocked(safeInvoke).mockResolvedValueOnce(false);
 await wrapper.findAll('.capture-tools button')[1]!.trigger('click');await flushPromises();
 expect(wrapper.text()).toContain('已取消截图');expect(safeInvoke).not.toHaveBeenCalledWith('plugin_open_window',expect.anything());wrapper.unmount();
});
