import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { parseActivePort, discoverEndpoint } from "./cdp-endpoint.mjs";

test("reads the actual port and browser identity with Windows newlines", () => {
  assert.deepEqual(parseActivePort("53741\r\n/devtools/browser/abc-123\r\n"), {
    baseUrl: "http://127.0.0.1:53741", browserPath: "/devtools/browser/abc-123",
  });
});

test("rejects partial files, invalid ports and injected endpoint paths", () => {
  for (const text of ["", "12345", "0\n/devtools/browser/abc", "65536\n/devtools/browser/abc", "12345\nhttps://evil.example", "NaN\n/devtools/browser/abc"]) {
    assert.throws(() => parseActivePort(text));
  }
});

test("discovers WebView2 EBWebView profile through the launch session", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "mytools-cdp-"));
  try {
    const profile = path.join(root, "profile");
    await mkdir(path.join(profile, "EBWebView"), { recursive: true });
    await writeFile(path.join(profile, "EBWebView", "DevToolsActivePort"), "53741\n/devtools/browser/session-a");
    const sessionFile = path.join(root, "session.json");
    await writeFile(sessionFile, JSON.stringify({ profile }));
    const endpoint = await discoverEndpoint({ sessionFile });
    assert.equal(endpoint.baseUrl, "http://127.0.0.1:53741");
    assert.equal(endpoint.browserPath, "/devtools/browser/session-a");
    assert.deepEqual(await discoverEndpoint({ port: "19222", sessionFile }), { baseUrl: "http://127.0.0.1:19222" });
  } finally { await rm(root, { recursive: true, force: true }); }
});

test("does not silently fall back to a hardcoded port", async () => {
  await assert.rejects(discoverEndpoint({ sessionFile: path.join(os.tmpdir(), "mytools-missing-session", "session.json") }), /dev:debug/);
  await assert.rejects(discoverEndpoint({ port: "0" }), /--port/);
});
