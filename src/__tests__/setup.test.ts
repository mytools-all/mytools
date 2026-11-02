/**
 * @vitest-environment happy-dom
 */
import { describe, it, expect } from "vitest";

/**
 * Sanity test to prove the Vitest pipeline is wired up before any real
 * logic lands. Delete once real tests exist.
 */
describe("vitest pipeline", () => {
  it("runs a trivial assertion", () => {
    expect(1 + 1).toBe(2);
  });

  it("has a DOM environment available", () => {
    const el = document.createElement("div");
    el.textContent = "hello";
    expect(el.textContent).toBe("hello");
  });
});
