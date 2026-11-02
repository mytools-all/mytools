import { describe, it, expect, vi, beforeEach } from "vitest";
import { reportPluginError } from "@/composables/pluginBridge";
import { reportSystemError } from "@/composables/systemErrors";

vi.mock("@/composables/pluginBridge", () => ({
  reportPluginError: vi.fn(),
}));

describe("reportSystemError", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("forwards to reportPluginError with system: prefix and an error shape", () => {
    reportSystemError("webview", "WebView 子进程异常退出 3 次", "trace here");
    expect(reportPluginError).toHaveBeenCalledWith("system:webview", {
      kind: "error",
      message: "WebView 子进程异常退出 3 次",
      stack: "trace here",
    });
  });

  it("omits stack when not provided", () => {
    reportSystemError("settings", "load failed");
    expect(reportPluginError).toHaveBeenCalledWith("system:settings", {
      kind: "error",
      message: "load failed",
      stack: undefined,
    });
  });
});
