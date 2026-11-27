import { describe, expect, it } from "vitest";
import { pluginCompatibility } from "./pluginCompatibility";

describe("plugin host requirements", () => {
  it("enforces the stable 0.2 boundary and gives the right update direction",()=>{
    const plugin={engines:{mytools:">=0.2.0 <0.3.0"}};
    expect(pluginCompatibility(plugin,"0.1.9").action).toBe("host");
    for(const version of ["0.2.0","0.2.1","0.2.99"])expect(pluginCompatibility(plugin,version).compatible).toBe(true);
    for(const version of ["0.3.0","1.0.0"])expect(pluginCompatibility(plugin,version)).toMatchObject({compatible:false,action:"plugin"});
    expect(pluginCompatibility(plugin,"0.3.0-beta.1").compatible).toBe(false);
  });
  it("treats plugin and host versions independently and supports npm ranges", () => {
    expect(pluginCompatibility({ engines: { mytools: ">=0.2.0 <1.0.0" } }, "0.2.0").compatible).toBe(true);
    expect(pluginCompatibility({ engines: { mytools: ">=0.2.0 <1.0.0" } }, "1.0.0").compatible).toBe(false);
    expect(pluginCompatibility({ engines: { mytools: "^0.2.0 || ^1.0.0" } }, "1.0.1").compatible).toBe(true);
  });
  it("permits legacy plugins and rejects invalid declarations or unavailable APIs", () => {
    expect(pluginCompatibility({})).toMatchObject({ compatible: true, declared: false });
    expect(pluginCompatibility({ engines: { mytools: "" } }).compatible).toBe(false);
    expect(pluginCompatibility({ engines: { mytools: "latest" } }).compatible).toBe(false);
    expect(pluginCompatibility({ apiVersion: 2 }).compatible).toBe(false);
  });
});
