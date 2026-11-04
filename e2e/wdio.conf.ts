import type { Options } from "@wdio/types";
import { browser } from "@wdio/globals";
import { mkdir } from "node:fs/promises";
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
  afterTest: async function (_test, _context, { passed }) {
    const directory = process.env.MYTOOLS_E2E_ARTIFACTS;
    if (passed || !directory) return;
    await mkdir(directory, {recursive:true});
    await browser.saveScreenshot(join(directory, `failure-${Date.now()}.png`));
  },
};
