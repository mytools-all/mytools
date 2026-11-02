import { describe, it, expect, beforeEach, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory, type Router } from "vue-router";
import { mount } from "@vue/test-utils";
import { encodeRouteTarget, useResultActions } from "@/composables/useResultActions";
import { useSearchStore, type SearchResult } from "@/stores/search";

const usageMock = {
  recent: [] as string[],
  pinned: [] as string[],
  bump: vi.fn(),
  togglePinned: vi.fn(),
};

vi.mock("@/stores/usage", () => ({
  useUsageStore: () => usageMock,
}));

vi.mock("@/lib/tauri", () => ({
  safeInvoke: vi.fn(async () => "ok"),
  tauriAvailable: () => false,
}));

function makeRouter(): Router {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/", name: "launcher", component: { template: "<div />" } },
      { path: "/tool/json-formatter", name: "json-formatter", component: { template: "<div />" } },
      { path: "/tool/base64-url", name: "base64-url", component: { template: "<div />" } },
      { path: "/tool/timestamp-uuid", name: "timestamp-uuid", component: { template: "<div />" } },
      { path: "/tool/color-picker", name: "color-picker", component: { template: "<div />" } },
      { path: "/tool/clipboard", name: "clipboard", component: { template: "<div />" } },
      { path: "/plugin/:id/:code", name: "plugin", component: { template: "<div />" } },
      { path: "/plugins", name: "plugins", component: { template: "<div />" } },
    ],
  });
}

function result(over: Partial<SearchResult>): SearchResult {
  return { kind: "x", id: "", title: "t", subtitle: "s", score: 1, ...over };
}

describe("encodeRouteTarget", () => {
  it("encodes query params as a query string", () => {
    expect(encodeRouteTarget("/plugins", { section: "settings" })).toBe(
      "/plugins?section=settings",
    );
  });

  it("encodes multiple params", () => {
    expect(encodeRouteTarget("/plugins", { section: "installed", page: "2" })).toBe(
      "/plugins?section=installed&page=2",
    );
  });

  it("omits the question mark when there is no query", () => {
    expect(encodeRouteTarget("/plugins")).toBe("/plugins");
  });
});

describe("useResultActions", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    usageMock.bump.mockReset();
    usageMock.togglePinned.mockReset();
  });

  async function setup() {
    const router = makeRouter();
    await router.push("/");
    await router.isReady();
    return { router, search: useSearchStore() };
  }

  /** The composable needs an active component instance for useRouter(). */
  function withSetup<T>(fn: () => T, router: Router): T {
    let out!: T;
    const Comp = {
      setup() {
        out = fn();
        return () => null;
      },
    };
    mount(Comp, { global: { plugins: [router] } });
    return out;
  }

  it("navigates route results using the encoded query string", async () => {
    const { router } = await setup();
    const { run } = withSetup(() => useResultActions(), router);
    await run(result({ kind: "route", id: "/plugins?section=settings" }));
    expect(router.currentRoute.value.name).toBe("plugins");
    expect(router.currentRoute.value.query.section).toBe("settings");
  });

  it("resets the query after a route command", async () => {
    const { router, search } = await setup();
    const { run } = withSetup(() => useResultActions(), router);
    search.query = "偏好设置";
    await run(result({ kind: "route", id: "/plugins?section=settings" }));
    expect(search.query).toBe("");
  });

  it("falls back to the launcher for malformed route ids", async () => {
    const { router } = await setup();
    const { run } = withSetup(() => useResultActions(), router);
    await run(result({ kind: "route", id: "not-a-path" }));
    expect(router.currentRoute.value.name).toBe("launcher");
  });

  it("navigates builtin tool results", async () => {
    const { router } = await setup();
    const { run } = withSetup(() => useResultActions(), router);
    for (const [kind, name] of [
      ["json-formatter", "json-formatter"],
      ["base64-url", "base64-url"],
      ["timestamp-uuid", "timestamp-uuid"],
      ["color-picker", "color-picker"],
      ["clipboard", "clipboard"],
    ] as const) {
      await run(result({ kind, id: kind }));
      expect(router.currentRoute.value.name).toBe(name);
    }
  });

  it("opens plugin results and bumps usage", async () => {
    const { router } = await setup();
    const { run } = withSetup(() => useResultActions(), router);
    await run(result({ kind: "plugin", id: "demo:index" }));
    expect(router.currentRoute.value.name).toBe("plugin");
    expect(router.currentRoute.value.params).toMatchObject({
      id: "demo",
      code: "index",
    });
    expect(usageMock.bump).toHaveBeenCalledWith("demo");
  });

  it("ignores plugin results with a malformed id", async () => {
    const { router } = await setup();
    const { run } = withSetup(() => useResultActions(), router);
    await run(result({ kind: "plugin", id: "no-colon" }));
    expect(router.currentRoute.value.name).toBe("launcher");
    expect(usageMock.bump).not.toHaveBeenCalled();
  });

  it("sends store results to the plugin store section", async () => {
    const { router } = await setup();
    const { run } = withSetup(() => useResultActions(), router);
    await run(result({ kind: "store", id: "store" }));
    expect(router.currentRoute.value.name).toBe("plugins");
    expect(router.currentRoute.value.query.section).toBe("store");
  });

  it("sends plugin manager results to the installed section", async () => {
    const { router } = await setup();
    const { run } = withSetup(() => useResultActions(), router);
    await run(result({ kind: "plugins", id: "plugins" }));
    expect(router.currentRoute.value.name).toBe("plugins");
    expect(router.currentRoute.value.query.section).toBe("installed");
  });

  it("clears the query after entering a plugin", async () => {
    const { router, search } = await setup();
    const { run } = withSetup(() => useResultActions(), router);
    search.query = "markdown";
    await run(result({ kind: "plugin", id: "demo:index" }));
    expect(search.query).toBe("");
    expect(router.currentRoute.value.name).toBe("plugin");
  });

  it("clears the query after opening a builtin tool", async () => {
    const { router, search } = await setup();
    const { run } = withSetup(() => useResultActions(), router);
    search.query = "json";
    await run(result({ kind: "json-formatter", id: "json-formatter" }));
    expect(search.query).toBe("");
    expect(router.currentRoute.value.name).toBe("json-formatter");
  });

  it("keeps the query after a calculator copy", async () => {
    const { router, search } = await setup();
    const { run } = withSetup(() => useResultActions(), router);
    search.query = "=1+2";
    await run(result({ kind: "calculator", id: "calculator:inline", title: "3" }));
    expect(search.query).toBe("=1+2");
  });

  it("detachSelected no-ops without a plugin result", async () => {
    const { router, search } = await setup();
    const { detachSelected } = withSetup(() => useResultActions(), router);
    search.results = [result({ kind: "app", id: "C:/x.exe" })];
    search.selectedIndex = 0;
    search.query = "app";
    await detachSelected();
    expect(search.query).toBe("app");
  });

  it("detachSelected clears the query after opening the window", async () => {
    const { router, search } = await setup();
    const { detachSelected } = withSetup(() => useResultActions(), router);
    search.results = [result({ kind: "plugin", id: "demo:index" })];
    search.selectedIndex = 0;
    search.query = "markdown";
    await detachSelected();
    expect(search.query).toBe("");
  });
});