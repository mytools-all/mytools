import { describe, expect, it } from "vitest";
import { pluginCompatibility } from "./pluginCompatibility";

describe("plugin host requirements", () => {
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
