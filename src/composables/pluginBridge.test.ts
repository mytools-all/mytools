import { callAsPlugin } from "@/composables/mytools";
import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  PLUGIN_BRIDGE_CLIENT_SCRIPT,
  addPluginErrorNoisePattern,
  clearPluginErrors,
  getPluginErrorNoisePatterns,
  reportPluginError,
  resetPluginErrorNoiseFilter,
  setPluginErrorNoiseFilter,
  usePluginErrors,
  usePluginIframeBridge,
} from "@/composables/pluginBridge";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => null),
}));

vi.mock("@/composables/mytools", () => ({
  callAsPlugin: vi.fn(async () => "result"),
  openPluginSession: vi.fn(async () => "session-demo"),
  closePluginSession: vi.fn(async () => {}),
  readEnv: () => ({}),
}));

describe("plugin error bus", () => {
  beforeEach(() => {
    resetPluginErrorNoiseFilter();
    clearPluginErrors("demo:run");
  });

  it("stores entries per key and clears them", () => {
    const errors = usePluginErrors();
    reportPluginError("demo:run", { kind: "error", message: "a" });
    reportPluginError("demo:run", { kind: "error", message: "b" });
    expect(errors.forKey("demo:run").map((e) => e.message)).toEqual(["a", "b"]);
    errors.clear("demo:run");
    expect(errors.forKey("demo:run")).toEqual([]);
  });

  it("filters the default ResizeObserver noise but keeps other errors", () => {
    const errors = usePluginErrors();
    reportPluginError("demo:run", {
      kind: "error",
      message: "ResizeObserver loop completed with undelivered notifications",
    });
    reportPluginError("demo:run", {
      kind: "error",
      message: "ReferenceError: x is not defined",
    });
    expect(errors.forKey("demo:run")).toHaveLength(1);
  });

  it("supports custom noise patterns", () => {
    const errors = usePluginErrors();
    addPluginErrorNoisePattern(/MyCustomNoise/);
    reportPluginError("demo:run", { kind: "error", message: "MyCustomNoise" });
    reportPluginError("demo:run", { kind: "error", message: "RealError" });
    expect(errors.forKey("demo:run")).toHaveLength(1);
    expect(getPluginErrorNoisePatterns().length).toBeGreaterThan(1);
  });

  it("can disable filtering entirely", () => {
    const errors = usePluginErrors();
    setPluginErrorNoiseFilter(null);
    reportPluginError("demo:run", {
      kind: "error",
      message: "ResizeObserver loop completed with undelivered notifications",
    });
    expect(errors.forKey("demo:run")).toHaveLength(1);
    resetPluginErrorNoiseFilter();
    errors.clear("demo:run");
  });
});

describe("plugin iframe bridge", () => {
  it("forwards capability calls to the host and posts the reply", async () => {
    const iframe = document.createElement("iframe");
    Object.defineProperty(iframe, "src", {
      value: "http://127.0.0.1:1234/token/demo/index.html",
    });

    const posted: unknown[] = [];
    const source = {
      postMessage: (payload: unknown) => posted.push(payload),
    } as unknown as Window;
    Object.defineProperty(iframe, "contentWindow", { value: source });

    const bridge = usePluginIframeBridge({ scope: "demo" });
    bridge.install(iframe);

    window.dispatchEvent(
      new MessageEvent("message", {
        data: {
          $mytoolsCall: {
            id: 7,
            capability: "clipboard.write",
            args: { text: "x" },
          },
        },
        source: source as unknown as MessageEventSource,
        origin: "http://127.0.0.1:1234",
      }),
    );
    await new Promise((r) => setTimeout(r, 0));

    expect(callAsPlugin).toHaveBeenCalledWith(
      "demo",
      "clipboard.write",
      { text: "x" },
      "session-demo",
      expect.any(AbortSignal),
    );
    expect(posted.length).toBeGreaterThan(0);
    expect(JSON.stringify(posted[0])).toContain("$mytoolsReply");
    bridge.teardown();
    iframe.remove();
  });

  it("reports plugin page errors to the error sink", () => {
    const iframe = document.createElement("iframe");
    Object.defineProperty(iframe, "src", {
      value: "http://127.0.0.1:1234/token/demo/index.html",
    });

    Object.defineProperty(iframe, "contentWindow", {
      value: { postMessage: vi.fn() },
    });
    const reportError = vi.fn();
    const bridge = usePluginIframeBridge({ reportError });
    bridge.install(iframe);
    window.dispatchEvent(
      new MessageEvent("message", {
        data: { $mytools: { kind: "error", message: "boom" } },
        source: iframe.contentWindow,
        origin: "http://127.0.0.1:1234",
      }),
    );
    expect(reportError).toHaveBeenCalledWith({
      kind: "error",
      message: "boom",
    });
    bridge.teardown();
  });

  it("rejects foreign windows, null sources and wrong origins", () => {
    const iframe = document.createElement("iframe");
    Object.defineProperty(iframe, "src", {
      value: "http://127.0.0.1:1234/demo",
    });

    const reportError = vi.fn();
    vi.mocked(callAsPlugin).mockClear();
    const bridge = usePluginIframeBridge({ reportError, scope: "demo" });
    bridge.install(iframe);
    for (const [source, origin] of [
      [window, "http://127.0.0.1:1234"],
      [null, "http://127.0.0.1:1234"],
      [iframe.contentWindow, "https://evil.example"],
    ] as const) {
      window.dispatchEvent(
        new MessageEvent("message", {
          source,
          origin,
          data: {
            $mytools: { kind: "error", message: "forged" },
            $mytoolsCall: { id: 1, capability: "storage.get" },
          },
        }),
      );
    }
    expect(reportError).not.toHaveBeenCalled();
    expect(callAsPlugin).not.toHaveBeenCalled();
    bridge.teardown();
    iframe.remove();
  });

  it("keeps the mounted identity and ignores messages after teardown", async () => {
    const iframe = document.createElement("iframe");
    Object.defineProperty(iframe, "src", {
      value: "http://127.0.0.1:1234/demo",
    });
    const source = { postMessage: vi.fn() } as unknown as Window;
    Object.defineProperty(iframe, "contentWindow", { value: source });
    let scope = "demo";
    const bridge = usePluginIframeBridge({ scope: () => scope });
    bridge.install(iframe);
    scope = "other";
    vi.mocked(callAsPlugin).mockClear();
    const event = new MessageEvent("message", {
      source,
      origin: "http://127.0.0.1:1234",
      data: { $mytoolsCall: { id: 2, capability: "storage.get", args: {} } },
    });
    window.dispatchEvent(event);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(callAsPlugin).toHaveBeenCalledWith(
      "demo",
      "storage.get",
      {},
      "session-demo",
      expect.any(AbortSignal),
    );
    bridge.teardown();
    vi.mocked(callAsPlugin).mockClear();
    window.dispatchEvent(event);
    expect(callAsPlugin).not.toHaveBeenCalled();
  });

  it("sends lifecycle events into the iframe", () => {
    const iframe = document.createElement("iframe");
    Object.defineProperty(iframe, "src", {
      value: "http://127.0.0.1:1234/token/demo/index.html",
    });
    const postMessage = vi.fn();
    Object.defineProperty(iframe, "contentWindow", { value: { postMessage } });
    const bridge = usePluginIframeBridge();
    bridge.install(iframe);
    bridge.send("enter", { code: "run" });
    expect(postMessage).toHaveBeenCalledWith(
      { $mytoolsEvent: { event: "enter", payload: { code: "run" } } },
      "http://127.0.0.1:1234",
    );
    bridge.teardown();
  });
});

describe("bridge client script", () => {
  it("keeps rejection details visible when older clients report reason", () => {
    const key = "markdown:rejection-regression";
    reportPluginError(key, {
      kind: "unhandledrejection",
      message: "",
      reason: "保存被拒绝",
    });
    expect(usePluginErrors().forKey(key)[0].message).toBe("保存被拒绝");
    clearPluginErrors(key);
  });

  it("reports a message for rejected promises without a reason", () => {
    const messages: unknown[] = [];
    const listeners: Record<string, (event: unknown) => void> = {};
    const win = {
      parent: { postMessage: (message: unknown) => messages.push(message) },
      addEventListener: (name: string, listener: (event: unknown) => void) => {
        listeners[name] = listener;
      },
    };
    new Function("window", PLUGIN_BRIDGE_CLIENT_SCRIPT)(win);
    listeners.unhandledrejection({ reason: undefined });
    expect(messages).toContainEqual({
      $mytools: {
        kind: "unhandledrejection",
        message: "Promise rejected without a reason",
        reason: "Promise rejected without a reason",
        stack: "",
      },
    });
  });
  it("exposes window.mytools with the capability call channel", () => {
    expect(PLUGIN_BRIDGE_CLIENT_SCRIPT).toContain("window.mytools");
    expect(PLUGIN_BRIDGE_CLIENT_SCRIPT).toContain("$mytoolsCall");
    expect(PLUGIN_BRIDGE_CLIENT_SCRIPT).toContain("$mytoolsEvent");
  });

  it("no longer ships the ecosystem proxy", () => {
    expect(PLUGIN_BRIDGE_CLIENT_SCRIPT).not.toContain("window.rubick");
    expect(PLUGIN_BRIDGE_CLIENT_SCRIPT).not.toContain("makeProxy");
  });

  it("uses the shared bridge client resource embedded by the Rust server", async () => {
    const rustPluginServerSource =
      await import("../../src-tauri/src/commands/plugin_server.rs?raw");
    const bridgeClientSource =
      await import("../../src-tauri/resources/bridge-client.js?raw");
    expect(PLUGIN_BRIDGE_CLIENT_SCRIPT).toBe(bridgeClientSource.default);
    expect(rustPluginServerSource.default as string).toContain(
      'const BRIDGE_CLIENT: &str = include_str!("../../resources/bridge-client.js");',
    );
  });

  it("forwards iframe errors to the host", () => {
    const messages: unknown[] = [];
    const registered: Record<string, (e: unknown) => void> = {};
    const win = {
      addEventListener(type: string, cb: (e: unknown) => void) {
        registered[type] = cb;
      },
      parent: {
        postMessage(payload: unknown) {
          messages.push(payload);
        },
      },
    };
    const fn = new Function("window", PLUGIN_BRIDGE_CLIENT_SCRIPT);
    fn(win);

    const onError = registered["error"];
    expect(typeof onError).toBe("function");
    onError?.({
      message: "boom",
      filename: "x.js",
      lineno: 3,
      colno: 4,
      error: { stack: "stack-line" },
    });
    expect(messages).toEqual([
      {
        $mytools: {
          kind: "error",
          message: "boom",
          filename: "x.js",
          lineno: 3,
          colno: 4,
          stack: "stack-line",
        },
      },
    ]);
  });

  it("rejects forged parent messages and accepts the expected parent origin", () => {
    let receive: (event: unknown) => void = () => {};
    const parent = {};
    const win = {
      parent,
      addEventListener(type: string, callback: (event: unknown) => void) {
        if (type === "message") receive = callback;
      },
    };
    new Function("window", "document", PLUGIN_BRIDGE_CLIENT_SCRIPT)(win, {
      referrer: "http://tauri.localhost/index.html",
    });
    const api = (
      win as unknown as { mytools: { onPluginEnter: (cb: () => void) => void } }
    ).mytools;
    const enter = vi.fn();
    api.onPluginEnter(enter);
    const data = {
      $mytoolsEvent: { event: "enter", payload: { plugin: "demo" } },
    };
    receive({ source: {}, origin: "http://tauri.localhost", data });
    receive({ source: parent, origin: "https://evil.example", data });
    expect(enter).not.toHaveBeenCalled();
    receive({ source: parent, origin: "http://tauri.localhost", data });
    expect(enter).toHaveBeenCalledOnce();
  });

  it("suppresses ResizeObserver loop noise", () => {
    const messages: unknown[] = [];
    const registered: Record<string, (e: unknown) => void> = {};
    const win = {
      addEventListener(type: string, cb: (e: unknown) => void) {
        registered[type] = cb;
      },
      parent: {
        postMessage(payload: unknown) {
          messages.push(payload);
        },
      },
    };
    const fn = new Function("window", PLUGIN_BRIDGE_CLIENT_SCRIPT);
    fn(win);
    registered["error"]?.({
      message: "ResizeObserver loop completed with undelivered notifications",
    });
    expect(messages).toHaveLength(0);
  });
});
