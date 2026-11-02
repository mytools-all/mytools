type InternalsWindow = { __TAURI_INTERNALS__?: unknown };

export function tauriAvailable(): boolean {
  return Boolean((window as unknown as InternalsWindow).__TAURI_INTERNALS__);
}

export async function safeInvoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T | null> {
  if (!tauriAvailable()) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<T>(cmd, args);
}