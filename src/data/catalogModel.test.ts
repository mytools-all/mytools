import { describe, expect, it } from "vitest";
import snapshot from "./catalogSnapshot.json";
import { parseToolCatalog } from "./catalogModel";

describe("catalog index validation", () => {
  it("maps index metadata to immutable-version npm installs", () => {
    const entries = parseToolCatalog(snapshot);
    expect(entries).toHaveLength(13);
    const demo = entries.find(entry => entry.name === "mytools-demo")!;
    expect(demo.keywords).toContain("mytools-demo");
    expect(demo.replaces).toContain("demo-hello");
    expect(demo.npm).toBe("@mytools-all/mytools-demo");
  });
  it("rejects duplicate identities and executable or ambiguous install sources", () => {
    expect(() => parseToolCatalog({...snapshot, plugins: [snapshot.plugins[0], snapshot.plugins[0]]})).toThrow();
    for (const patch of [{npm: "https://example.com/evil.tgz"}, {version: "latest"}, {icon: "javascript:alert(1)"}, {published: "true"}]) {
      expect(() => parseToolCatalog({...snapshot, plugins: [{...snapshot.plugins[0], ...patch}]})).toThrow();
    }
    expect(() => parseToolCatalog({...snapshot, schemaVersion: 999})).toThrow();
  });
});
