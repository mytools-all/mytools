import type { Options } from "@wdio/types";
import { browser } from "@wdio/globals";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

type TauriCapability = WebdriverIO.Capabilities & { "tauri:options": { application: string } };

export const config: Options.Testrunner & { capabilities: TauriCapability[] } = {
  runner: "local",
  specs: ["./**/*.test.ts"],
  framework: "mocha",
  reporters: ["spec"],
  maxInstances: 1,
  logLevel: "info",
  protocol: "http",
  hostname: "127.0.0.1",
  port: 4440,
  path: "/",
  connectionRetryCount: 0,
  capabilities: [
    {
      browserName: "mytools",
      "wdio:enforceWebDriverClassic": true,
      "tauri:options": {
        application:
          process.env.MYTOOLS_E2E_EXE ??
          join(process.cwd(), "src-tauri", "target", "release", "mytools.exe"),
      },
    },
  ],
  mochaOpts: {
    ui: "bdd",
    timeout: 30_000,
  },
  afterHook: async function (_test, _context, { error }) {
    const directory=process.env.MYTOOLS_E2E_ARTIFACTS;
    if(!error || !directory)return;
    await mkdir(directory,{recursive:true});
    const name=join(directory,`hook-failure-${Date.now()}`);
    await Promise.allSettled([
      browser.saveScreenshot(`${name}.png`),
      browser.getPageSource().then(source=>writeFile(`${name}.html`,source)),
      Promise.all([browser.getUrl(),browser.getTitle()]).then(([url,title])=>writeFile(`${name}.json`,JSON.stringify({url,title},null,2))),
    ]);
  },
  afterTest: async function (_test, _context, { passed }) {
    const directory = process.env.MYTOOLS_E2E_ARTIFACTS;
    if (passed || !directory) return;
    await mkdir(directory, {recursive:true});
    await browser.saveScreenshot(join(directory, `failure-${Date.now()}.png`));
  },
};
