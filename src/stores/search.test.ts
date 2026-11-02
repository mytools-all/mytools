import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import {
  useSearchStore,
  matchRouteCommand,
  routeCommandResults,
  ROUTE_COMMANDS,
} from "@/stores/search";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (_cmd: string, args: { query: string }) => {
    if (args.query === "=1+2") return [{ title: "3", subtitle: "= 1+2", kind: "calculator", id: "x", score: 1 }];
    if (args.query === "json") return [{ title: "JSON Formatter", subtitle: "type: json-formatter", kind: "json-formatter", id: "json-formatter", score: 100 }];
    return [];
  }),
}));

(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};

describe("search store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useFakeTimers();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("starts empty", () => {
    const s = useSearchStore();
    expect(s.results).toEqual([]);
    expect(s.hasResults).toBe(false);
    expect(s.selected).toBe(null);
  });

  it("wraps selection around the result list", async () => {
    const s = useSearchStore();
    s.query = "json";
    vi.advanceTimersByTime(60);
    await vi.runOnlyPendingTimersAsync();
    expect(s.results.length).toBe(1);
    s.move(1);
    expect(s.selectedIndex).toBe(0);
    s.move(-1);
    expect(s.selectedIndex).toBe(0);
  });

  it("surfaces calculator results via invoke", async () => {
    const s = useSearchStore();
    s.query = "=1+2";
    vi.advanceTimersByTime(60);
    await vi.runOnlyPendingTimersAsync();
    expect(s.results[0].title).toBe("3");
    expect(s.results[0].kind).toBe("calculator");
  });

  it("refreshNow returns fresh results without waiting for the debounce", async () => {
    const s = useSearchStore();
    s.query = "json";
    expect(vi.mocked(invoke)).not.toHaveBeenCalled();
    await s.refreshNow();
    expect(s.results.length).toBe(1);
    expect(s.results[0].title).toBe("JSON Formatter");
    expect(vi.mocked(invoke)).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(100);
    await vi.runOnlyPendingTimersAsync();
    expect(vi.mocked(invoke)).toHaveBeenCalledTimes(1);
  });

  it("still refreshes through the debounce when Enter is not pressed", async () => {
    const s = useSearchStore();
    s.query = "json";
    vi.advanceTimersByTime(60);
    await vi.runOnlyPendingTimersAsync();
    expect(s.results.length).toBe(1);
    expect(vi.mocked(invoke)).toHaveBeenCalledTimes(1);
  });

  it("resets state", () => {
    const s = useSearchStore();
    s.query = "abc";
    s.selectedIndex = 2;
    s.reset();
    expect(s.query).toBe("");
    expect(s.selectedIndex).toBe(0);
  });
});

describe("route commands", () => {
  it("matches 插件商店 to the store section", () => {
    expect(matchRouteCommand("插件商店")).toEqual({
      path: "/plugins",
      query: { section: "store" },
    });
  });

  it("matches 设置 / 偏好设置 / settings to the settings section", () => {
    for (const kw of ["设置", "偏好设置", "settings"]) {
      expect(matchRouteCommand(kw)).toEqual({
        path: "/plugins",
        query: { section: "settings" },
      });
    }
  });

  it("matches 插件中心 / 插件管理 to the installed section", () => {
    for (const kw of ["插件中心", "插件管理"]) {
      expect(matchRouteCommand(kw)?.query?.section).toBe("installed");
    }
  });

  it("trims surrounding whitespace", () => {
    expect(matchRouteCommand("  插件商店  ")?.query?.section).toBe("store");
  });

  it("fuzzily matches partial / prefixed queries to the best keyword", () => {
    expect(matchRouteCommand("偏好")?.query?.section).toBe("settings");
    expect(matchRouteCommand("打开偏好设置")?.query?.section).toBe("settings");
    expect(matchRouteCommand("打开插件商店")?.query?.section).toBe("store");
    expect(matchRouteCommand("插件商店在哪")?.query?.section).toBe("store");
    expect(matchRouteCommand("settings页面")?.query?.section).toBe("settings");
  });

  it("picks the longest keyword when several hit", () => {
    expect(matchRouteCommand("设置")).toEqual({
      path: "/plugins",
      query: { section: "settings" },
    });
    expect(matchRouteCommand("打开偏好设置")?.query?.section).toBe("settings");
  });

  it("returns null for empty or non-matching input", () => {
    expect(matchRouteCommand("")).toBeNull();
    expect(matchRouteCommand("   ")).toBeNull();
    expect(matchRouteCommand("json")).toBeNull();
    expect(matchRouteCommand("wibble")).toBeNull();
    expect(matchRouteCommand("设")).toBeNull();
  });

  it("every command targets the plugins route", () => {
    for (const target of Object.values(ROUTE_COMMANDS)) {
      expect(target.path).toBe("/plugins");
    }
  });
});

describe("route command results", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("surfaces 偏好设置 as a visible route result", async () => {
    const s = useSearchStore();
    s.query = "偏好设置";
    vi.advanceTimersByTime(60);
    await vi.runOnlyPendingTimersAsync();
    expect(s.results).toHaveLength(1);
    expect(s.results[0].kind).toBe("route");
    expect(s.results[0].id).toBe("/plugins?section=settings");
    expect(s.results[0].title).toBe("偏好设置");
  });

  it("surfaces 插件商店 as a visible route result", async () => {
    const s = useSearchStore();
    s.query = "插件商店";
    vi.advanceTimersByTime(60);
    await vi.runOnlyPendingTimersAsync();
    expect(s.results[0].kind).toBe("route");
    expect(s.results[0].id).toBe("/plugins?section=store");
  });

  it("returns no route row when the query is not a route command", () => {
    expect(routeCommandResults("json")).toEqual([]);
    expect(routeCommandResults("  ")).toEqual([]);
    expect(routeCommandResults("wibble")).toEqual([]);
  });

  it("surfaces fuzzy route matches as a visible row", () => {
    const rows = routeCommandResults("打开偏好设置");
    expect(rows).toHaveLength(1);
    expect(rows[0].kind).toBe("route");
    expect(rows[0].id).toBe("/plugins?section=settings");
    expect(rows[0].title).toBe("打开偏好设置");
    expect(routeCommandResults("偏好")[0].id).toBe("/plugins?section=settings");
    expect(routeCommandResults("插件中心")[0].id).toBe("/plugins?section=installed");
  });

  it("renders one identical icon for every alias of the same target", () => {
    const icon = (q: string) => routeCommandResults(q)[0]?.icon;
    const settingsIcon = icon("设置");
    expect(settingsIcon).toBeTruthy();
    expect(icon("偏好设置")).toBe(settingsIcon);
    expect(icon("settings")).toBe(settingsIcon);
    expect(icon("打开偏好设置")).toBe(settingsIcon);
    const pluginsIcon = icon("插件中心");
    expect(pluginsIcon).toBeTruthy();
    expect(icon("插件管理")).toBe(pluginsIcon);
    const storeIcon = icon("插件商店");
    expect(storeIcon).toBeTruthy();
    expect(storeIcon).not.toBe(settingsIcon);
    expect(storeIcon).not.toBe(pluginsIcon);
  });

  it("still returns the route row when the backend is unavailable", async () => {
    const prev = (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    const s = useSearchStore();
    s.query = "偏好设置";
    vi.advanceTimersByTime(60);
    await vi.runOnlyPendingTimersAsync();
    expect(s.results).toHaveLength(1);
    expect(s.results[0].kind).toBe("route");
    if (prev === undefined) {
      delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    } else {
      (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = prev;
    }
  });
});