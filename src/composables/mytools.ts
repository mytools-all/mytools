/**
 * Host API surface — `window.mytools.*`.
 *
 * Every host capability call goes through the backend capability registry
 * (`plugin_capability_call`), which owns permission policy and dispatch.
 * Plugin pages get the same surface through the injected bridge client
 * (see `composables/pluginBridge.ts`).
 *
 * See docs/02-architecture.md §4 and docs/04-plugin-spec.md §7.
 */
import { invoke } from "@tauri-apps/api/core";
import { askCapabilityConsent } from "@/runtime/capabilityConsent";
export { resolveCapabilityConsent, useCapabilityConsent } from "@/runtime/capabilityConsent";
export type { CapabilityConsent } from "@/runtime/capabilityConsent";
import { tauriAvailable } from "@/lib/tauri";

export const MYTOOLS_API_VERSION = 1;

export type MyToolsErrorCode =
  | "CAPABILITY_DENIED"
  | "CAPABILITY_UNKNOWN"
  | "API_VERSION"
  | "INVALID_ARGS"
  | "HANDLER_FAILED"
  | "UNAVAILABLE";

export interface MyToolsError {
  code: MyToolsErrorCode;
  message: string;
  capability?: string;
  /** The host needs the user's consent before the call may run. */
  consentRequired?: boolean;
}

export type MyToolsEnvType =
  | "text"
  | "img"
  | "files"
  | "regex"
  | "over"
  | "window";

export interface MyToolsEnv {
  code: string;
  type: MyToolsEnvType;
  payload?: unknown;
  plugin?: string;
  apiVersion: number;
}

export interface MyToolsAPI {
  readonly apiVersion: number;
  call<T = unknown>(
    capability: string,
    args?: Record<string, unknown>,
  ): Promise<T>;
  getEnv(): MyToolsEnv;
  onPluginReady(cb: (env: MyToolsEnv) => void): void;
  onPluginEnter(cb: (env: MyToolsEnv) => void): void;
  onPluginOut(cb: () => void): void;
  onFilesDropped?(cb: () => void): void;
  onSearch?(cb: (event: {query:string;key?:string}) => void): void;
}

type LifecycleEvent = "ready" | "enter" | "out";

/** Parse the plugin context out of the current route. */
export function readEnv(): MyToolsEnv {
  const hash = window.location.hash;
  const frag = hash.slice(hash.lastIndexOf("#") + 1).replace(/^\//, "");
  const env: MyToolsEnv = {
    code: "default",
    type: "text",
    apiVersion: MYTOOLS_API_VERSION,
  };
  if (frag.startsWith("plugin/")) {
    const parts = frag.split("/");
    env.plugin = parts[1] ?? "";
    env.code = parts[2] ?? "default";
    return env;
  }
  const params = new URLSearchParams(frag);
  if (params.has("feature")) {
    env.code = params.get("feature") ?? env.code;
    env.plugin = params.get("plugin") ?? undefined;
  }
  return env;
}

function toError(err: unknown, capability: string): MyToolsError {
  if (err && typeof err === "object") {
    const candidate = err as {
      code?: unknown;
      message?: unknown;
      capability?: unknown;
      consentRequired?: unknown;
    };
    if (typeof candidate.code === "string") {
      return {
        code: candidate.code as MyToolsErrorCode,
        message:
          typeof candidate.message === "string"
            ? candidate.message
            : `capability ${capability} failed`,
        capability:
          typeof candidate.capability === "string"
            ? candidate.capability
            : capability,
        consentRequired: candidate.consentRequired === true,
      };
    }
  }
  return {
    code: "HANDLER_FAILED",
    message: err instanceof Error ? err.message : String(err),
    capability,
  };
}

let installed: MyToolsAPI | null = null;
let dispatchRef: ((event: LifecycleEvent, env: MyToolsEnv) => void) | null = null;

/**
 * Call a host capability on behalf of a specific plugin.
 *
 * The iframe bridge uses this: the plugin that asked lives in another
 * window, so the scope cannot be derived from the host route alone.
 */
export async function callAsPlugin<T = unknown>(
  scope: string | null,
  capability: string,
  args?: Record<string, unknown>,
  session?: string,
  signal?: AbortSignal,
): Promise<T> {
  if (!scope) throw { code: "CAPABILITY_DENIED", message: "missing plugin identity", capability };
  return callWithScope<T>(scope, capability, args, session, signal);
}

export async function openPluginSession(plugin: string): Promise<string> {
  if (!tauriAvailable()) throw new Error("host API is unavailable");
  return invoke<string>("plugin_session_open", { plugin });
}

export async function closePluginSession(session: string): Promise<void> {
  await invoke("plugin_session_close", { session });
}

async function callWithScope<T = unknown>(
  scope: string | null,
  capability: string,
  args?: Record<string, unknown>,
  session?: string,
  signal?: AbortSignal,
): Promise<T> {
  if (!tauriAvailable()) {
    throw {
      code: "UNAVAILABLE",
      message: "host API is not available outside the desktop app",
      capability,
    } satisfies MyToolsError;
  }
  const ensureActive = () => {
    if (signal?.aborted) throw { code: "CAPABILITY_DENIED", message: "plugin session closed", capability } satisfies MyToolsError;
  };
  ensureActive();
  const payload = {
    capability,
    args: args ?? null,
    scope,
    ...(scope ? { session: session ?? await openPluginSession(scope) } : {}),
  };
  try {
    ensureActive();
    return await invoke<T>("plugin_capability_call", payload);
  } catch (err) {
    const error = toError(err, capability);
    // The host asks for consent; ask the user, record the grant and
    // retry once.
    if (error.consentRequired && scope) {
      const granted = await askCapabilityConsent({
        capability,
        plugin: scope,
        message: error.message,
      }, signal);
      if (granted) {
        ensureActive();
        await invoke("plugin_grant_capability", {
          plugin: scope,
          capability,
        });
        ensureActive();
        return await invoke<T>("plugin_capability_call", payload);
      }
    }
    throw error;
  }
}

/** Dispatch a lifecycle event to every registered handler. */
export function emitMyToolsLifecycle(
  event: LifecycleEvent,
  env?: Partial<MyToolsEnv>,
) {
  const base = readEnv();
  dispatchRef?.(event, { ...base, ...env });
}

/**
 * Install `window.mytools` (idempotent). Call once at app startup,
 * before the ecosystem shim is installed.
 */
export function installMyToolsApi(): MyToolsAPI {
  if (installed) return installed;

  const ready: Array<(env: MyToolsEnv) => void> = [];
  const enter: Array<(env: MyToolsEnv) => void> = [];
  const out: Array<() => void> = [];

  const api: MyToolsAPI = {
    apiVersion: MYTOOLS_API_VERSION,

    async call<T = unknown>(
      capability: string,
      args?: Record<string, unknown>,
    ): Promise<T> {
      return callWithScope<T>(readEnv().plugin ?? null, capability, args);
    },

    getEnv() {
      return readEnv();
    },

    onPluginReady(cb) {
      ready.push(cb);
    },

    onPluginEnter(cb) {
      enter.push(cb);
    },

    onPluginOut(cb) {
      out.push(cb);
    },
  };

  dispatchRef = (event, env) => {
    if (event === "ready") {
      for (const cb of ready) cb(env);
    } else if (event === "enter") {
      for (const cb of enter) cb(env);
    } else {
      for (const cb of out) cb();
    }
  };

  installed = api;
  (window as unknown as Record<string, unknown>).mytools = api;
  return api;
}

/** The installed API, if any. */
export function useMyTools(): MyToolsAPI | null {
  return installed;
}
