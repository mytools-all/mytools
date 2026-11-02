import { reportPluginError } from "@/composables/pluginBridge";

/**
 * Unified entry point for framework-level errors (webview health,
 * settings load failure, plugin install failure, tray creation
 * failure, …). They all funnel into the same `usePluginErrors`
 * channel the existing launcher error panel renders.
 */
export function reportSystemError(
  source: string,
  message: string,
  stack?: string,
): void {
  reportPluginError(`system:${source}`, {
    kind: "error",
    message,
    stack,
  });
}
