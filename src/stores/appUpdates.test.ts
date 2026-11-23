import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { flushPromises } from "@vue/test-utils";
import { useAppUpdatesStore, type UpdateSnapshot } from "./appUpdates";

const { invoke, subscribe } = vi.hoisted(() => ({ invoke: vi.fn(), subscribe: vi.fn() }));
vi.mock("@/lib/tauri", () => ({ tauriAvailable: () => true, safeInvoke: invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: subscribe }));

function snapshot(values: Partial<UpdateSnapshot> = {}): UpdateSnapshot {
  return { currentVersion: "0.2.0", latestVersion: null, status: "unknown", mode: "notify", checkedAt: null, error: null, phase: "idle", downloadedBytes: 0, totalBytes: null, canInstall: false, releaseUrl: null, revision: 1, ...values };
}

describe("主程序更新状态", () => {
  beforeEach(() => {
    setActivePinia(createPinia()); invoke.mockReset(); subscribe.mockReset();
    subscribe.mockResolvedValue(vi.fn());
    invoke.mockImplementation(async command => command === "app_autostart_status" ? false : snapshot());
  });

  it("初始化订阅和状态只加载一次，不由页面重复发起检查", async () => {
    const store = useAppUpdatesStore();
    await Promise.all([store.initialize(), store.initialize()]);
    expect(subscribe).toHaveBeenCalledTimes(1);
    expect(invoke.mock.calls.filter(([command]) => command === "app_update_status")).toHaveLength(1);
    expect(invoke).not.toHaveBeenCalledWith("app_update_check");
    expect(store.autostart).toBe(false);
  });

  it("检查失败保留上次版本但不能显示已经最新，忽略较旧的异步响应", async () => {
    const store = useAppUpdatesStore(); await store.initialize();
    const handler = subscribe.mock.calls[0]![1];
    handler({ payload: snapshot({ latestVersion: "0.3.0", status: "available", revision: 3 }) });
    expect(store.showReminder).toBe(true);
    handler({ payload: snapshot({ latestVersion: "0.1.0", status: "upToDate", revision: 2 }) });
    expect(store.state.latestVersion).toBe("0.3.0");
    handler({ payload: snapshot({ latestVersion: "0.3.0", status: "error", error: "网络失败", revision: 4 }) });
    expect(store.statusText).toContain("检查失败");
    expect(store.statusText).not.toContain("最新版了");
  });

  it("保存策略或自启失败时保持原来的真实状态", async () => {
    const store = useAppUpdatesStore(); await store.initialize();
    invoke.mockRejectedValue(Error("无法写入"));
    await store.setAutostart(true); await store.setMode("download");
    expect(store.autostart).toBe(false); expect(store.state.mode).toBe("notify");
    expect(store.autostartError).toContain("无法写入"); expect(store.error).toContain("无法写入");
  });

  it("自动下载完成仍等待确认，稍后提醒只隐藏当前版本", async () => {
    const store = useAppUpdatesStore(); await store.initialize();
    const handler = subscribe.mock.calls[0]![1];
    handler({ payload: snapshot({ latestVersion: "0.3.0", status: "available", mode: "download", phase: "ready", revision: 2 }) });
    expect(store.statusText).toContain("等待安装"); expect(invoke).not.toHaveBeenCalledWith("app_update_install", expect.anything());
    store.dismiss(); expect(store.showReminder).toBe(false);
    handler({ payload: snapshot({ latestVersion: "0.4.0", status: "available", revision: 3 }) });
    expect(store.showReminder).toBe(true);
  });

  it("销毁时释放尚未返回的事件订阅", async () => {
    let resolve: ((value: () => void) => void) | undefined;
    const unlisten = vi.fn();
    subscribe.mockReturnValue(new Promise<() => void>(done => { resolve = done; }));
    const store = useAppUpdatesStore(); void store.initialize(); store.dispose();
    resolve?.(unlisten); await flushPromises();
    expect(unlisten).toHaveBeenCalledOnce();
    expect(invoke).not.toHaveBeenCalled();
  });
});
