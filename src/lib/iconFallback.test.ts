import { describe, it, expect } from "vitest";
import { fallbackIconDataUri } from "@/lib/iconFallback";

describe("fallbackIconDataUri", () => {
  it("returns a data: URI starting with image/svg+xml", () => {
    const u = fallbackIconDataUri("rubick-plugin-demo");
    expect(u.startsWith("data:image/svg+xml")).toBe(true);
  });

  it("uses the first character as glyph (uppercased)", () => {
    const u = fallbackIconDataUri("demo");
    expect(decodeURIComponent(u)).toMatch(/>D</);
  });

  it("strips scope from @scoped/name and uses next char", () => {
    const u = fallbackIconDataUri("@scope/awesome");
    expect(decodeURIComponent(u)).toMatch(/>A</);
  });

  it("produces deterministic output for the same name", () => {
    expect(fallbackIconDataUri("foo")).toBe(fallbackIconDataUri("foo"));
  });

  it("differs across names", () => {
    expect(fallbackIconDataUri("alpha")).not.toBe(fallbackIconDataUri("beta"));
  });

  it("escapes XML-sensitive characters in glyph", () => {
    const u = fallbackIconDataUri("<weird&name>");
    const svg = decodeURIComponent(u);
    expect(svg).not.toContain("<weird&name>");
    expect(svg).toContain("&lt;");
  });

  it("escapes & glyph", () => {
    const u = fallbackIconDataUri("&name");
    const svg = decodeURIComponent(u);
    expect(svg).toContain("&amp;");
    expect(svg).not.toContain(">&</text>");
  });

  it("uses a fallback glyph for empty name", () => {
    const u = fallbackIconDataUri("");
    expect(decodeURIComponent(u)).toMatch(/>\?</);
  });
});