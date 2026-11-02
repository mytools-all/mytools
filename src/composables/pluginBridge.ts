/**
 * Plugin bridge — the host side of the plugin iframe channel.
 *
 * A plugin runs inside an iframe served by the local plugin server. The
 * injected client script (`src-tauri/resources/bridge-client.js`) gives
 * the plugin `window.mytools.*`; capability calls travel over
 * `postMessage` and are executed by the backend capability registry.
 * The same channel carries plugin page errors so the host can surface
 * them.
 *
 * See docs/02-architecture.md §4 and docs/04-plugin-spec.md §7.
 */
import { callAsPlugin, closePluginSession, openPluginSession, readEnv as readMyToolsEnv } from "@/composables/mytools";
import bridgeClientRaw from "../../src-tauri/resources/bridge-client.js?raw";

export * from "@/runtime/pluginErrors";
import type { PluginErrorPayload } from "@/runtime/pluginErrors";
export type LifecycleEvent = "ready" | "enter" | "out" | "files" | "search";

interface BridgeMessage {
  $mytools?: PluginErrorPayload;
  $mytoolsCall?: { id?: number; capability?: string; args?: unknown };
  $mytoolsReply?: { id?: number; ok?: boolean; value?: unknown; error?: string };
}

export interface BridgeOptions {
  reportError?: (payload: PluginErrorPayload) => void;
  /** Plugin id the iframe belongs to; used as capability scope. */
  scope?: string | (() => string | undefined);
}

/**
 * Host side of the plugin iframe bridge. Forwards capability calls from
 * a plugin iframe to the backend registry and posts the result back.
 * `opts.reportError` receives `$mytools` envelopes reported by the
 * injected client script (iframe `error` / `unhandledrejection`).
 */
export function usePluginIframeBridge(opts: BridgeOptions = {}) {
  const { reportError } = opts;
  const cleanups: Array<() => void> = [];
  let currentFrame: HTMLIFrameElement | null = null;

  function install(iframe: HTMLIFrameElement | null) {
    cleanups.forEach((f) => f());
    cleanups.length = 0;
    currentFrame = iframe;
    if (!iframe) return;

    const scopeOption = opts.scope;
    const scope = (typeof scopeOption === "function" ? scopeOption() : scopeOption)
      ?? readMyToolsEnv().plugin ?? null;
    let origin: string;
    try {
      origin = new URL(iframe.src).origin;
    } catch {
      return;
    }
    if (origin === "null") return;
    let disposed = false;
    const controller = new AbortController();
    const session = scope ? openPluginSession(scope).then(
      (value) => ({ value }), () => ({ value: undefined }),
    ) : Promise.resolve({ value: undefined });
    cleanups.push(() => {
      disposed = true;
      controller.abort();
      void session.then(({ value }) => {
        return value ? closePluginSession(value) : undefined;
      }).catch(() => {});
    });

    const handler = (event: MessageEvent<BridgeMessage>) => {
      if (disposed || !iframe.contentWindow || event.source !== iframe.contentWindow
        || event.origin !== origin) return;
      const hostCall = event.data?.$mytoolsCall;
      if (hostCall && typeof hostCall.capability === "string"
        && typeof hostCall.id === "number" && Number.isSafeInteger(hostCall.id) && hostCall.id > 0
        && (hostCall.args == null || (typeof hostCall.args === "object" && !Array.isArray(hostCall.args)))) {
        const source = iframe.contentWindow;
        void dispatchHostCall(
          {
            id: hostCall.id,
            capability: hostCall.capability,
            args: hostCall.args,
          },
          source,
        );
        return;
      }
      const mytools = event.data?.$mytools;
      if (mytools && (mytools.kind === "error" || mytools.kind === "unhandledrejection")) {
        reportError?.(mytools);
      }
    };

    async function dispatchHostCall(
      msg: { id: number; capability: string; args?: unknown },
      source: MessageEventSource | null,
    ) {
      const reply = (ok: boolean, value?: unknown, error?: string) => {
        if (!source || disposed) return;
        (source as Window).postMessage(
          { $mytoolsReply: { id: msg.id, ok, value, error } },
          origin,
        );
      };
      const args =
        msg.args && typeof msg.args === "object"
          ? (msg.args as Record<string, unknown>)
          : undefined;
      try {
        const binding = await session;
        if (disposed) return;
        if (!binding.value) throw new Error("plugin session is unavailable");
        const value = await callAsPlugin(scope, msg.capability, args, binding.value, controller.signal);
        reply(true, value);
      } catch (e) {
        const message =
          e && typeof e === "object" && "message" in e
            ? String((e as { message?: unknown }).message)
            : String(e);
        reply(false, undefined, message);
      }
    }

    window.addEventListener("message", handler);
    cleanups.push(() => window.removeEventListener("message", handler));
  }

  function teardown() {
    cleanups.forEach((f) => f());
    cleanups.length = 0;
    currentFrame = null;
  }

  /** Send a host lifecycle event into the plugin iframe. */
  function send(event: LifecycleEvent, payload?: unknown) {
    const el = currentFrame;
    if (!el) return;
    const win = (el.contentWindow as Window | null) ?? null;
    if (!win) return;
    const origin = new URL(el.src).origin;
    if (origin !== "null") win.postMessage({ $mytoolsEvent: { event, payload } }, origin);
  }

  return { install, teardown, send };
}

/**
 * The injected client script (single source of truth in
 * `src-tauri/resources/bridge-client.js`; the Rust plugin server embeds
 * it with `include_str!`).
 */
export const PLUGIN_BRIDGE_CLIENT_SCRIPT: string = bridgeClientRaw;
