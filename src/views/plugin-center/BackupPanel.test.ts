import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia } from "pinia";
import BackupPanel from "./BackupPanel.vue";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mocks.open, save: mocks.save }));
const plugin = { id: "calendar", title: "日历", version: "0.1.0" };
describe("backup confirmation", () => {
  beforeEach(() => {
    mocks.invoke.mockReset(); mocks.open.mockReset(); mocks.save.mockReset();
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "backup_info") return { plugins: [plugin], lastRestore: null };
      if (command === "backup_preview") return { token: "preview-one", hasExistingData: true, warnings: [], manifest: { hostVersion: "0.2.0", createdAt: "2026-11-24T00:00:00Z", plugins: [plugin], filesCount: 3, hasHost: false, hasClipboard: false, hasPrograms: false } };
      return undefined;
    });
    mocks.open.mockResolvedValue("C:\\backup.zip");
  });
  it("previews without restoring and defaults the overwrite dialog to cancel", async () => {
    const wrapper = mount(BackupPanel, { attachTo: document.body, global: { plugins: [createPinia()] } });
    await flushPromises();
    await wrapper.findAll("button").find(button => button.text() === "选择 ZIP 备份…")!.trigger("click");
    await flushPromises();
    expect(mocks.invoke).not.toHaveBeenCalledWith("backup_restore", expect.anything());
    await wrapper.findAll("button").find(button => button.text() === "恢复所选内容…")!.trigger("click");
    await flushPromises();
    expect(document.activeElement?.textContent).toBe("取消");
    expect(document.querySelector('[role="alertdialog"]')?.textContent).toContain("当前已有数据");
    (document.activeElement as HTMLButtonElement).click();
    await flushPromises();
    expect(mocks.invoke).not.toHaveBeenCalledWith("backup_restore", expect.anything());
    wrapper.unmount();
    expect(mocks.invoke).toHaveBeenCalledWith("backup_cancel_preview", { token: "preview-one" });
  });
  it("restores only checked items after explicit confirmation", async () => {
    const wrapper = mount(BackupPanel, { attachTo: document.body, global: { plugins: [createPinia()] } });
    await flushPromises();
    await wrapper.findAll("button").find(button => button.text() === "选择 ZIP 备份…")!.trigger("click");
    await flushPromises();
    await wrapper.findAll("button").find(button => button.text() === "恢复所选内容…")!.trigger("click");
    await flushPromises();
    const confirm = [...document.querySelectorAll<HTMLButtonElement>('[role="alertdialog"] button')].find(button => button.textContent === "确认覆盖并重启")!;
    confirm.click(); await flushPromises();
    expect(mocks.invoke).toHaveBeenCalledWith("backup_restore", { token: "preview-one", confirmed: true, options: { host: false, clipboard: false, plugins: ["calendar"], programs: false } });
    wrapper.unmount();
  });
});
