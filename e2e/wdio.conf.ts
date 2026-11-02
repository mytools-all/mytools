import type { Options } from "@wdio/types";

export const config: Options.Testrunner = {
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
  capabilities: [
    {
      browserName: "mytools",
      "tauri:options": {
        application:
          process.env.MYTOOLS_E2E_EXE ??
          `${process.env.LOCALAPPDATA}\\MyTools\\mytools.exe`,
      },
    },
  ],
  mochaOpts: {
    ui: "bdd",
    timeout: 30_000,
  },
};