#!/usr/bin/env node
import http from "node:http";
import { fileURLToPath } from "node:url";
import { discoverEndpoint } from "./cdp-endpoint.mjs";

const args = process.argv.slice(2);
const expression = args[0];
if (!expression) {
  console.error("usage: node scripts/cdp-eval.mjs \"<expression>\" [--frame <sub>] [--target main|iframe] [--timeout ms] [--port port] [--profile path] [--window label]");
  process.exit(2);
}

function option(name, fallback) {
  const index = args.indexOf(name);
  if (index === -1 || index === args.length - 1) return fallback;
  return args[index + 1];
}

const frameFilter = option("--frame", "");
const targetKind = option("--target", frameFilter ? "iframe" : "main");
const timeoutMs = Number(option("--timeout", "15000"));
const windowLabel = option("--window", "main");
let endpoint;

function readJson(url) {
  return new Promise((resolve, reject) => {
    const request = http.get(url, (res) => {
      let body = "";
      res.on("data", (chunk) => { body += chunk; });
      res.on("error", reject);
      res.on("end", () => {
        try {
          if (res.statusCode !== 200) throw new Error(`HTTP ${res.statusCode}`);
          resolve(JSON.parse(body));
        } catch (error) { reject(error); }
      });
    });
    request.setTimeout(3000, () => request.destroy(new Error("CDP HTTP request timed out")));
    request.on("error", reject);
  });
}

async function listTargets() {
  endpoint = await discoverEndpoint({
    port: option("--port", undefined),
    profile: option("--profile", undefined),
    sessionFile: fileURLToPath(new URL("../src-tauri/target/cdp-session.json", import.meta.url)),
  });
  if (endpoint.browserPath) {
    const version = await readJson(`${endpoint.baseUrl}/json/version`);
    if (new URL(version.webSocketDebuggerUrl).pathname !== endpoint.browserPath) {
      throw new Error("CDP session is stale; restart with pnpm dev:debug");
    }
  }
  const targets = await readJson(`${endpoint.baseUrl}/json/list`);
  if (targetKind === "iframe" && !targets.some((target) => target.type === "iframe")) {
    const parent = targets.find((target) => target.type === "page");
    if (parent) {
      const client = await connect(parent.webSocketDebuggerUrl);
      try {
        const response = await client.send("Target.getTargets", {});
        for (const target of response.result?.targetInfos ?? []) {
          if (target.type !== "iframe") continue;
          const owner = targets.find((page) => page.id === target.parentId) ?? parent;
          targets.push({ ...target, parentWebSocketDebuggerUrl: owner.webSocketDebuggerUrl });
        }
      } finally { client.close(); }
    }
  }
  return targets;
}

function connect(url) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    const pending = new Map();
    let nextId = 0;
    ws.addEventListener("message", (event) => {
      const msg = JSON.parse(event.data);
      if (msg.id && pending.has(msg.id)) {
        pending.get(msg.id)(msg);
        pending.delete(msg.id);
      }
    });
    ws.addEventListener("open", () =>
      resolve({
        send(method, params, sessionId) {
          return new Promise((res) => {
            const id = ++nextId;
            pending.set(id, res);
            ws.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }));
          });
        },
        close: () => ws.close(),
      }),
    );
    ws.addEventListener("error", reject);
  });
}

async function evaluate(ws, expr) {
  return ws.send("Runtime.evaluate", {
    expression: expr,
    returnByValue: true,
    awaitPromise: false,
  });
}

async function pickTarget(targets) {
  const pages = targets.filter((t) => t.type === "page");
  const iframes = targets.filter((t) => t.type === "iframe");

  if (targetKind === "iframe") {
    const hit =
      frameFilter ? iframes.find((t) => (t.url || "").includes(frameFilter)) : iframes[0];
    if (!hit) throw new Error("no iframe target found");
    return hit;
  }

  if (frameFilter) {
    const hit = iframes.find((t) => (t.url || "").includes(frameFilter));
    if (!hit) throw new Error(`no iframe matching "${frameFilter}"`);
    return hit;
  }

  for (const page of pages) {
    const ws = await connect(page.webSocketDebuggerUrl);
    try {
      const probe = await evaluate(ws, "window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label");
      if (probe.result?.result?.value === windowLabel) return page;
    } finally { ws.close(); }
  }
  throw new Error(`Could not find window ${windowLabel}`);
}

async function main() {
  const targets = await listTargets();
  const target = await pickTarget(targets);
  let ws;
  if (target.parentWebSocketDebuggerUrl) {
    const parent = await connect(target.parentWebSocketDebuggerUrl);
    const attached = await parent.send("Target.attachToTarget", { targetId: target.targetId, flatten: true });
    if (!attached.result?.sessionId) { parent.close(); throw new Error("Could not attach plugin frame"); }
    ws = { send: (method, params) => parent.send(method, params, attached.result.sessionId), close: parent.close };
  } else {
    ws = await connect(target.webSocketDebuggerUrl);
  }
  await ws.send("Runtime.enable", {});

  const started = await evaluate(
    ws,
    `window.__cdp = 'pending'; (function(){ ${expression} })(); 'started'`,
  );
  if (started.result?.exceptionDetails) {
    console.error(
      "expression threw:",
      JSON.stringify(started.result.exceptionDetails.text),
    );
    ws.close();
    process.exit(1);
  }

  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, 150));
    const res = await evaluate(ws, "JSON.stringify(window.__cdp ?? null)");
    const raw = res.result?.result?.value;
    if (raw === undefined || raw === null) continue;
    const value = JSON.parse(raw);
    if (value === "pending") continue;
    console.log(typeof value === "string" ? value : JSON.stringify(value));
    ws.close();
    process.exit(0);
  }

  console.error(
    "TIMEOUT — did the expression assign window.__cdp? (WebView2 cannot await promises)",
  );
  ws.close();
  process.exit(1);
}

main().catch((err) => {
  console.error(String(err?.message || err));
  process.exit(1);
});
