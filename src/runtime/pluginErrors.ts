import { ref } from "vue";

export interface PluginErrorPayload {
  kind: "error" | "unhandledrejection";
  message: string;
  filename?: string;
  lineno?: number;
  colno?: number;
  stack?: string;
  reason?: string;
}

export interface PluginError extends PluginErrorPayload {
  at: number;
}

export const DEFAULT_NOISE_PATTERNS: RegExp[] = [
  /ResizeObserver loop completed with undelivered notifications/i,
  /ResizeObserver loop limit exceeded/i,
];

const ERROR_HISTORY_LIMIT = 20;

interface NoiseFilter {
  patterns: RegExp[];
}

let noiseFilters: NoiseFilter | null = {
  patterns: DEFAULT_NOISE_PATTERNS.map((rx) => new RegExp(rx.source, rx.flags)),
};

function isNoisePayload(payload: PluginErrorPayload): boolean {
  if (!noiseFilters) return false;
  const text = payload.message ?? "";
  return noiseFilters.patterns.some((rx) => rx.test(text));
}

export function setPluginErrorNoiseFilter(patterns: RegExp[] | null) {
  if (patterns === null) {
    noiseFilters = null;
    return;
  }
  noiseFilters = { patterns: patterns.map((rx) => new RegExp(rx.source, rx.flags)) };
}

export function addPluginErrorNoisePattern(rx: RegExp) {
  if (!noiseFilters) {
    noiseFilters = { patterns: [rx] };
    return;
  }
  noiseFilters.patterns.push(new RegExp(rx.source, rx.flags));
}

export function resetPluginErrorNoiseFilter() {
  noiseFilters = {
    patterns: DEFAULT_NOISE_PATTERNS.map((rx) => new RegExp(rx.source, rx.flags)),
  };
}

export function getPluginErrorNoisePatterns(): RegExp[] {
  return noiseFilters ? noiseFilters.patterns.slice() : [];
}

const pluginErrorsByKey = new Map<string, PluginError[]>();
const pluginErrorsVersion = ref(0);

export function reportPluginError(key: string, payload: PluginErrorPayload) {
  payload = { ...payload, message: payload.message || payload.reason || "插件发生异步错误，未提供详细原因" };
  if (isNoisePayload(payload)) {
    if (typeof console !== "undefined" && console.debug) {
      console.debug("[plugin error filtered]", payload);
    }
    return;
  }
  const list = pluginErrorsByKey.get(key) ?? [];
  list.push({ ...payload, at: Date.now() });
  if (list.length > ERROR_HISTORY_LIMIT) list.splice(0, list.length - ERROR_HISTORY_LIMIT);
  pluginErrorsByKey.set(key, list);
  pluginErrorsVersion.value++;
}

export function clearPluginErrors(key: string) {
  if (!pluginErrorsByKey.has(key)) return;
  pluginErrorsByKey.delete(key);
  pluginErrorsVersion.value++;
}

export function usePluginErrors() {
  return {
    version: pluginErrorsVersion,
    forKey(key: string) {
      void pluginErrorsVersion.value;
      return pluginErrorsByKey.get(key) ?? [];
    },
    clear(key: string) {
      clearPluginErrors(key);
    },
  };
}
