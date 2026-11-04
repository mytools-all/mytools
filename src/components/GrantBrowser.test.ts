import { mount, flushPromises } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import GrantBrowser from "./GrantBrowser.vue";

vi.mock("@/stores/plugins",()=>({usePluginsStore:()=>({plugins:[{manifest:{name:"mytools-calc",pluginName:"计算稿纸"}},{manifest:{name:"mytools-ocr",pluginName:"文字识别"}}]})}));
const records = [{plugin:"mytools-calc",permission:"clipboard.write"},{plugin:"mytools-calc",permission:"fs.exporttext"},{plugin:"mytools-ocr",permission:"clipboard.write"}];

describe("grant browsing",()=>{
  function setup(directory=false) {
    const revoke=vi.fn(async()=>{});
    const wrapper=mount(GrantBrowser,{props:{title:"授权",description:"说明",records:directory ? records.map(record=>({...record,permission:"C:\\fixture"})) : records,loading:false,error:null,revoke,directory}});
    return {wrapper,revoke};
  }
  it("groups by plugin name and supports combined plugin and permission searches",async()=>{
    const {wrapper}=setup();
    expect(wrapper.findAll(".grants__group")).toHaveLength(2);
    expect(wrapper.text()).toContain("计算稿纸");
    await wrapper.get("input").setValue("计算 写入");
    expect(wrapper.findAll("li")).toHaveLength(1);
    expect(wrapper.text()).toContain("clipboard.write");
    await wrapper.get("input").setValue("not-found");
    expect(wrapper.findAll("li")).toHaveLength(0);
    expect(wrapper.text()).toContain("没有匹配的授权");
  });
  it("groups matching permissions and revokes only the chosen plugin's grant",async()=>{
    const {wrapper,revoke}=setup();
    await wrapper.get("select").setValue("permission");
    await wrapper.get("input").setValue("clipboard.write");
    expect(wrapper.findAll(".grants__group")).toHaveLength(1);
    expect(wrapper.findAll("li")).toHaveLength(2);
    await wrapper.get('[aria-label="撤销 计算稿纸 的 clipboard.write"]').trigger("click");
    await flushPromises();
    expect(revoke).toHaveBeenCalledOnce();
    expect(revoke).toHaveBeenCalledWith("mytools-calc","clipboard.write");
  });
  it("supports directory grouping and filtering by plugin and path",async()=>{
    const {wrapper}=setup(true);
    await wrapper.get("select").setValue("permission");
    expect(wrapper.findAll(".grants__group")).toHaveLength(1);
    await wrapper.get("input").setValue("文字 fixture");
    expect(wrapper.findAll("li")).toHaveLength(1);
    expect(wrapper.text()).toContain("文字识别");
  });
});
