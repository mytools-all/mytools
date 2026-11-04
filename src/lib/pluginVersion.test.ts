import { describe,it,expect } from "vitest";
import { comparePluginVersions as cmp } from "./pluginVersion";
describe("plugin version order",()=>{
 it("compares numeric releases and prerelease identifiers",()=>{expect(cmp("0.10.0","0.9.9")).toBe(1);expect(cmp("1.0.0","1.0.0-rc.1")).toBe(1);expect(cmp("1.0.0-beta.11","1.0.0-beta.2")).toBe(1);expect(cmp("1.0.0-beta.2","1.0.0-beta.x")).toBe(-1);expect(cmp("1.0.0+abc","1.0.0+xyz")).toBe(0);});
 it("does not offer updates for malformed versions",()=>{for(const version of ["latest","1.0","01.0.0","1.0.0-01"])expect(cmp(version,"1.0.0")).toBeNull();});
});
