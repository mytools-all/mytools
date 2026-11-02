import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import {
  armDragGuard,
  disarmDragGuard,
  isDragGuardActive,
  setupDragGuard,
} from "@/lib/dragGuard";

describe("dragGuard", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(1_000_000);
    disarmDragGuard();
  });

  afterEach(() => {
    vi.useRealTimers();
    disarmDragGuard();
  });

  it("is inactive by default", () => {
    expect(isDragGuardActive()).toBe(false);
  });

  it("is active right after arming", () => {
    armDragGuard();
    expect(isDragGuardActive()).toBe(true);
  });

  it("expires after the TTL window", () => {
    armDragGuard();
    vi.advanceTimersByTime(2001);
    expect(isDragGuardActive()).toBe(false);
  });

  it("is inactive after being disarmed", () => {
    armDragGuard();
    disarmDragGuard();
    expect(isDragGuardActive()).toBe(false);
  });

  it("re-arms and slides the deadline on each call", () => {
    armDragGuard();
    vi.advanceTimersByTime(1500);
    armDragGuard();
    vi.advanceTimersByTime(500);
    expect(isDragGuardActive()).toBe(true);
    vi.advanceTimersByTime(1501);
    expect(isDragGuardActive()).toBe(false);
  });

  it("tears down its listeners when returned fn is called", () => {
    const down = vi.fn();
    document.addEventListener("pointerdown", down, true);
    const cleanup = setupDragGuard();
    cleanup();
    const el = document.createElement("div");
    el.setAttribute("data-tauri-drag-region", "deep");
    document.body.appendChild(el);
    el.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    expect(isDragGuardActive()).toBe(false);
    document.body.removeChild(el);
    document.removeEventListener("pointerdown", down, true);
  });

  it("arms on pointerdown within a drag-region subtree", () => {
    setupDragGuard();
    const header = document.createElement("header");
    header.setAttribute("data-tauri-drag-region", "deep");
    document.body.appendChild(header);
    const child = document.createElement("span");
    header.appendChild(child);
    child.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    expect(isDragGuardActive()).toBe(true);
    document.body.removeChild(header);
  });

  it("does not arm on pointerdown outside a drag region", () => {
    setupDragGuard();
    const el = document.createElement("div");
    document.body.appendChild(el);
    el.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    expect(isDragGuardActive()).toBe(false);
    document.body.removeChild(el);
  });

  it("disarms on pointerup", () => {
    setupDragGuard();
    const header = document.createElement("header");
    header.setAttribute("data-tauri-drag-region", "deep");
    document.body.appendChild(header);
    header.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    expect(isDragGuardActive()).toBe(true);
    document.dispatchEvent(new PointerEvent("pointerup", { bubbles: true }));
    expect(isDragGuardActive()).toBe(false);
    document.body.removeChild(header);
  });
});