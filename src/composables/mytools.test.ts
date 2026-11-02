import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  installMyToolsApi,
  emitMyToolsLifecycle,
  readEnv,
  resolveCapabilityConsent,
  useCapabilityConsent,
  useMyTools,
  MYTOOLS_API_VERSION,
  type MyToolsAPI,
} from "@/composables/mytools";

const invokeSpy = vi.fn(
  async (_cmd: string, _args: unknown): Promise<unknown> => null,
);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeSpy(...(args as [string, unknown])),
}));

function mountTauri() {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" } },
    invoke: invokeSpy,
  };
}

describe("mytools host api", () => {
  let api: MyToolsAPI;

  beforeEach(() => {
    invokeSpy.mockReset();
    invokeSpy.mockImplementation(async (cmd) => cmd === "plugin_session_open" ? "session-demo" : null);
    mountTauri();
    window.location.hash = "#/";
    api = installMyToolsApi();
  });

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it("installs idempotently and exposes the api version", () => {
    expect(installMyToolsApi()).toBe(api);
    expect(api.apiVersion).toBe(MYTOOLS_API_VERSION);
    expect(useMyTools()).toBe(api);
  });

  it("calls the capability registry with capability, args and scope", async () => {
    window.location.hash = "#/plugin/demo/run";
    await api.call("clipboard.write", { text: "hi" });
    expect(invokeSpy).toHaveBeenCalledWith("plugin_capability_call", {
      capability: "clipboard.write",
      args: { text: "hi" },
      scope: "demo",
      session: "session-demo",
    });
  });

  it("sends a null scope outside a plugin route", async () => {
    await api.call("system.path", { name: "home" });
    expect(invokeSpy).toHaveBeenCalledWith("plugin_capability_call", {
      capability: "system.path",
      args: { name: "home" },
      scope: null,
    });
  });

  it("passes the backend value through", async () => {
    invokeSpy.mockImplementation(async () => "C:\\Users\\demo");
    await expect(api.call<string>("system.path")).resolves.toBe("C:\\Users\\demo");
  });

  it("normalizes a structured backend error", async () => {
    invokeSpy.mockImplementation(async () => {
      throw {
        code: "CAPABILITY_DENIED",
        message: "capability fs.read denied: requires user authorization",
        capability: "fs.read",
      };
    });
    await expect(api.call("fs.read")).rejects.toMatchObject({
      code: "CAPABILITY_DENIED",
      capability: "fs.read",
    });
  });

  it("wraps an unstructured failure as HANDLER_FAILED", async () => {
    invokeSpy.mockImplementation(async () => {
      throw new Error("boom");
    });
    await expect(api.call("clipboard.write")).rejects.toMatchObject({
      code: "HANDLER_FAILED",
      message: "boom",
      capability: "clipboard.write",
    });
  });

  it("reports UNAVAILABLE outside the desktop app", async () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    await expect(api.call("clipboard.write")).rejects.toMatchObject({
      code: "UNAVAILABLE",
    });
  });

  it("reads the plugin context from the route", () => {
    window.location.hash = "#/plugin/demo/run";
    expect(readEnv()).toMatchObject({ code: "run", plugin: "demo" });
    window.location.hash = "#/plugin/demo";
    expect(readEnv()).toMatchObject({ code: "default", plugin: "demo" });
  });

  it("dispatches enter and out to registered handlers", () => {
    const enter = vi.fn();
    const out = vi.fn();
    api.onPluginEnter(enter);
    api.onPluginOut(out);
    emitMyToolsLifecycle("enter", { code: "run", plugin: "demo" });
    expect(enter).toHaveBeenCalledWith(
      expect.objectContaining({ code: "run", plugin: "demo" }),
    );
    emitMyToolsLifecycle("out");
    expect(out).toHaveBeenCalledOnce();
  });

  it("asks for consent, records the grant and retries the call", async () => {
    window.location.hash = "#/plugin/demo/run";
    let refused = true;
    invokeSpy.mockImplementation(async (cmd: string, _args: unknown) => {
      if (cmd === "plugin_grant_capability") return null;
      if (cmd === "plugin_capability_call") {
        if (refused) {
          refused = false;
          throw {
            code: "CAPABILITY_DENIED",
            message: "capability fs.read requires user authorization",
            capability: "fs.read",
            consentRequired: true,
          };
        }
        return "allowed";
      }
      return null;
    });

    const pending = api.call<string>("fs.read");
    await vi.waitFor(() => {
      expect(useCapabilityConsent().consent.value?.capability).toBe("fs.read");
    });
    resolveCapabilityConsent(true);
    await expect(pending).resolves.toBe("allowed");

    const grantCalls = invokeSpy.mock.calls.filter(
      (c) => c[0] === "plugin_grant_capability",
    );
    expect(grantCalls).toHaveLength(1);
    expect(grantCalls[0][1]).toEqual({ plugin: "demo", capability: "fs.read" });
  });

  it("does not retry when the user denies consent", async () => {
    window.location.hash = "#/plugin/demo/run";
    invokeSpy.mockImplementation(async (cmd: string) => {
      if (cmd === "plugin_capability_call") {
        throw {
          code: "CAPABILITY_DENIED",
          message: "capability fs.read requires user authorization",
          capability: "fs.read",
          consentRequired: true,
        };
      }
      return null;
    });

    const pending = api.call("fs.read");
    await vi.waitFor(() => {
      expect(useCapabilityConsent().consent.value).not.toBeNull();
    });
    resolveCapabilityConsent(false);
    await expect(pending).rejects.toMatchObject({ code: "CAPABILITY_DENIED" });

    const grantCalls = invokeSpy.mock.calls.filter(
      (c) => c[0] === "plugin_grant_capability",
    );
    expect(grantCalls).toHaveLength(0);
    expect(useCapabilityConsent().consent.value).toBeNull();
  });
});
