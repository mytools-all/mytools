import { browser, expect, $ } from "@wdio/globals";

describe("MyTools smoke", () => {
  before(async () => {
    await browser.waitUntil(async () => browser.execute(() => Boolean((window as unknown as {__TAURI_INTERNALS__?: {invoke?: unknown}}).__TAURI_INTERNALS__?.invoke)), {timeout:15_000, timeoutMsg:"Tauri host did not become ready"});
    await browser.execute(async () => {
      const host = (window as unknown as {__TAURI_INTERNALS__: {invoke: (command:string) => Promise<unknown>}}).__TAURI_INTERNALS__;
      await host.invoke("window_show");
    });
  });

  it("launches the launcher window", async () => {
    const title = await browser.getTitle();
    expect(title).toContain("MyTools");
  });

  it("shows the search input on the launcher", async () => {
    const search = await $(".search-box");
    await search.waitForExist({ timeout: 5_000 });
    expect(await search.isDisplayed()).toBe(true);
  });

  it("calculates without requiring installed plugins", async () => {
    const search = await $(".search-box");
    await search.setValue("=1+1");
    const result = await $(".result-list__title");
    await result.waitForExist({ timeout: 10_000 });
    await expect(result).toHaveText("2");
  });

  it("opens the plugin center and shows market cards", async () => {
    const search = await $(".search-box");
    await search.setValue("插件商店");
    await browser.keys("Enter");
    const cards = await $(".pc__main .tc__card");
    await cards.waitForExist({ timeout: 15_000 });
    await expect(cards).toBeDisplayed();
    expect(await browser.getUrl()).toContain("#/plugins");
  });

  it("returns to the launcher with Backspace in an empty search", async () => {
    const search = await $(".search-box");
    await search.waitForExist({ timeout: 5_000 });
    await search.click();
    await search.clearValue();
    await browser.keys("Backspace");
    await browser.waitUntil(async () => !(await browser.getUrl()).includes("#/plugins"), {timeout:5000});
    await expect(search).toBeDisplayed();
  });
  it("closes the main window to tray and can show it again", async () => {
    await $("[data-testid='app-menu-btn']").click();
    await $(".md__item--danger").click();
    try {
      await browser.waitUntil(async () => browser.execute(async () => {
        const host = (window as unknown as {__TAURI_INTERNALS__: {invoke: (command: string, args?: unknown) => Promise<unknown>}}).__TAURI_INTERNALS__;
        return !(await host.invoke("plugin:window|is_visible", {label: "main"}));
      }), {timeout: 5000, timeoutMsg: "Main window remained visible after closing"});
    } finally {
      await browser.execute(async () => {
        const host = (window as unknown as {__TAURI_INTERNALS__: {invoke: (command: string) => Promise<unknown>}}).__TAURI_INTERNALS__;
        await host.invoke("window_show");
      });
    }
    await expect($(".search-box")).toBeDisplayed();
  });

});
