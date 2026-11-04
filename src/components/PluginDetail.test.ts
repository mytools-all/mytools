import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import { useToolCatalogStore } from "@/stores/toolCatalog";
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
  beforeEach(()=>{ localStorage.clear(); });
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
  it("requires confirmation and returns an uninstalled catalog plugin to its install action",async()=>{
    const plugin={manifest:{name:"mytools-search",version:"0.1.0",description:"search",features:[{code:"run",explain:"运行",cmds:["search"]}]},enabled:true,origin:"custom"} as LoadedPlugin;
    const {wrapper}=await setup("mytools-search",plugin);
    const plugins=usePluginsStore();
    const uninstall=vi.spyOn(plugins,"uninstall").mockImplementation(async()=>{plugins.plugins=[];return true;});
    await wrapper.find(".pd__actions .pd__uninstall").trigger("click");
    expect(uninstall).not.toHaveBeenCalled();
    await wrapper.find(".pd__confirm .pd__uninstall").trigger("click");
    await flushPromises();
    expect(uninstall).toHaveBeenCalledWith("mytools-search");
    expect(wrapper.find(".pd__primary").text()).toBe("安装");
    expect(wrapper.text()).toContain("数据仍保留");
    wrapper.unmount();
  });
  it("keeps the installed plugin and shows uninstall errors",async()=>{
    const plugin: LoadedPlugin={manifest:{name:"mytools-search",description:"search",main:"index.html",version:"0.1.0",features:[{code:"run",explain:"运行",cmds:[]}]},enabled:true,origin:"custom",declaredCapabilities:[],dir:"fixture",mainUrl:"https://example.com/index.html"};
    const {wrapper}=await setup("mytools-search",plugin);
    const plugins=usePluginsStore();
    vi.spyOn(plugins,"uninstall").mockImplementation(async()=>{plugins.error="file locked";return false;});
    await wrapper.find(".pd__actions .pd__uninstall").trigger("click");
    await wrapper.find(".pd__confirm .pd__uninstall").trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("file locked");
    expect(wrapper.find(".pd__primary").text()).toBe("打开");
    wrapper.unmount();
  });
});

it("shows installed-to-catalog versions and routes the update action through the catalog",async()=>{
 const plugin={manifest:{name:"mytools-search",version:"0.1.0",description:"search",main:"index.html",features:[{code:"run",explain:"运行",cmds:[]}]},enabled:true,origin:"custom",declaredCapabilities:[],dir:"fixture",mainUrl:"https://example.com/index.html"} as LoadedPlugin;
 const {wrapper}=await setup("mytools-search",plugin);const catalog=useToolCatalogStore();const existing=catalog.entries.find(e=>e.name==="mytools-search")!;catalog.entries=catalog.entries.map(e=>e.name===existing.name?{...e,version:"0.2.0",status:"ready" as const,npmSpec:e.npm+"@0.2.0"}:e);await flushPromises();
 expect(wrapper.text()).toContain("可更新");expect(wrapper.text()).toContain("0.1.0");const button=wrapper.findAll("button").find(b=>b.text()==="更新至 v0.2.0")!;expect(button).toBeDefined();const install=vi.spyOn(catalog,"install").mockResolvedValue("mytools-search");await button.trigger("click");expect(install).toHaveBeenCalledWith(expect.objectContaining({name:"mytools-search",version:"0.2.0"}));wrapper.unmount();
});
