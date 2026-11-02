import { browser, expect, $ } from "@wdio/globals";

describe("MyTools smoke", () => {
  it("launches the launcher window", async () => {
    await browser.pause(2000);
    const title = await browser.getTitle();
    expect(title).toContain("MyTools");
  });

  it("shows the search input on the launcher", async () => {
    const search = await $(".search-box");
    await search.waitForExist({ timeout: 5_000 });
    expect(await search.isDisplayed()).toBe(true);
  });

  it("searches for the json formatter", async () => {
    const search = await $(".search-box");
    await search.setValue("json");
    await browser.pause(500);
    const resultList = await $(".result-list");
    await resultList.waitForExist({ timeout: 5_000 });
  });

  it("opens the plugin center and shows market cards", async () => {
    await browser.url("tauri://localhost/#/plugins");
    await browser.pause(800);
    const header = await $(".tool__title");
    await header.waitForExist({ timeout: 5_000 });
    const text = await header.getText();
    expect(text).toContain("插件中心");
  });

  it("returns to the launcher on Escape", async () => {
    const search = await $(".search-box");
    await search.waitForExist({ timeout: 5_000 });
    await search.click();
    await browser.keys(["Escape"]);
    await browser.pause(500);
  });
});