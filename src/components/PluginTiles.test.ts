import { mount } from "@vue/test-utils";
import { describe, it, expect } from "vitest";
import PluginTiles from "./PluginTiles.vue";

describe("plugin tile context menu", () => {
  it("adds through the context menu without launching the plugin", async () => {
    const wrapper=mount(PluginTiles,{props:{items:[{id:"fixture",name:"示例"}],panelable:true}});
    await wrapper.find("li").trigger("contextmenu",{clientX:10,clientY:10});
    const button=[...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].find(b=>b.textContent==="添加到我的面板");
    expect(button).toBeTruthy();button?.click();
    expect(wrapper.emitted("panel")).toEqual([["fixture"]]);
    expect(wrapper.emitted("open")).toBeUndefined();
    wrapper.unmount();
  });
});
