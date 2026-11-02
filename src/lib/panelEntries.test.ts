import { describe, it, expect } from "vitest";
import { validPanelEntry, movePanelEntry, type PanelEntry } from "./panelEntries";

const first: PanelEntry = {id:"a",title:"文档",kind:"path",target:"C:\\docs"};
const second: PanelEntry = {id:"b",title:"网站",kind:"url",target:"https://example.com"};
describe("panel entries", () => {
  it("rejects executable URL schemes and relative file targets", () => {
    expect(validPanelEntry(second)).toBe(true);
    expect(validPanelEntry({...second,target:"javascript:alert(1)"})).toBe(false);
    expect(validPanelEntry({...first,target:"docs"})).toBe(false);
    expect(validPanelEntry(first)).toBe(true);
  });
  it("moves entries without dropping or duplicating them", () => {
    expect(movePanelEntry([first,second],"b","a")).toEqual([second,first]);
    expect(movePanelEntry([first,second],"missing","a")).toEqual([first,second]);
  });
});
