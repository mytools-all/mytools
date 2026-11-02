import { describe, it, expect, vi } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import PluginDetail from "./PluginDetail.vue";
import { usePluginsStore, type LoadedPlugin } from "@/stores/plugins";

vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn()}));
vi.mock("@tauri-apps/plugin-dialog",()=>({open:vi.fn()}));

async function setup(name:string, plugin?:LoadedPlugin) {
  const pinia=createPinia();setActivePinia(pinia);
  if(plugin)usePluginsStore().plugins=[plugin];
  const router=createRouter({history:createMemoryHistory(),routes:[{path:"/plugins",component:{template:"<div/>"}},{path:"/plugin/:id/:code",name:"plugin",component:{template:"<div/>"}}]});
  await router.push({path:"/plugins",query:{section:"installed",detail:name}});
  const wrapper=mount(PluginDetail,{props:{name},global:{plugins:[pinia,router]}});
  return {wrapper,router};
}

describe("plugin details",()=>{
  it("explains independent distribution and shows capabilities",async()=>{
    const {wrapper}=await setup("mytools-search");
    expect(wrapper.text()).toContain("通过 npm 安装独立插件包");
    expect(wrapper.text()).toContain("fs.globalsearch");
    expect(wrapper.text()).toContain("安装");
    expect(wrapper.text()).not.toContain("下载量");
    wrapper.unmount();
  });
  it("supports an installed plugin outside the built-in catalog and preserves the return section",async()=>{
    const plugin={manifest:{name:"fixture",pluginName:"自定义工具",version:"9.0.0",description:"自定义详情",features:[{code:"run",explain:"运行",cmds:["fixture"]}]},declaredCapabilities:["clipboard.write"],enabled:true,origin:"custom"} as LoadedPlugin;
    const {wrapper,router}=await setup("fixture",plugin);
    expect(wrapper.text()).toContain("自定义工具");
    expect(wrapper.text()).toContain("9.0.0");
    expect(wrapper.text()).toContain("fixture");
    await wrapper.find(".pd__back").trigger("click");await flushPromises();
    expect(router.currentRoute.value.query).toEqual({section:"installed"});
    wrapper.unmount();
  });
});
