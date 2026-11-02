import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { defineComponent, h, nextTick } from "vue";
import { createMemoryHistory, createRouter } from "vue-router";
import { createPinia, setActivePinia } from "pinia";

interface HealthLike {
  my_pid: number;
  live_pids: number[];
  crash_count: number;
  last_crash_at: number | null;
}

const invokeMock = vi.fn(async (_cmd: string, _args?: unknown): Promise<HealthLike | null> => null);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...(args as [string, unknown])),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

vi.mock("@/lib/tauri", () => ({
  safeInvoke: (cmd: string, args?: Record<string, unknown>) =>
    invokeMock(cmd, args) as Promise<HealthLike | null>,
  tauriAvailable: () => true,
}));

import { useWebviewHealth } from "@/composables/useWebviewHealth";
import { clearPluginErrors, usePluginErrors } from "@/composables/pluginBridge";

function makeProbe() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/", name: "launcher", component: defineComponent({ render: () => h("div") }) },
    ],
  });
  return mount(
    defineComponent({
      setup() {
        const probe = useWebviewHealth();
        return () =>
          h("div", { id: "probe" }, probe.health.value ? "ready" : "loading");
      },
    }),
    { global: { plugins: [router] } },
  );
}

describe("useWebviewHealth", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(null);
    clearPluginErrors("system:webview");
  });

  it("queries webview_health on mount", async () => {
    makeProbe();
    await nextTick();
    await flushPromises();
    const calls = invokeMock.mock.calls.map((c) => c[0]);
    expect(calls).toContain("webview_health");
  });

  it("reports a plugin error when crash_count > 0", async () => {
    invokeMock.mockResolvedValue({
      my_pid: 1,
      live_pids: [11],
      crash_count: 3,
      last_crash_at: Date.now(),
    });

    makeProbe();
    await flushPromises();
    const entries = usePluginErrors().forKey("system:webview");
    expect(entries.length).toBeGreaterThan(0);
    expect(entries[0].message).toContain("异常退出");
  });
});