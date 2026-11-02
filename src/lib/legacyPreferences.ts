/**
 * One-time migration from the pre-settings.json localStorage keys.
 *
 * This is the only place in the host that still touches localStorage:
 * it runs once during startup, copies the legacy preferences into
 * `settings.json`, and then clears the host origin's storage. After a
 * successful migration this module can be deleted.
 */

export interface LegacyPreferences {
  theme?: "system" | "light" | "dark";
  hotkey?: string;
  pinned?: string[];
  recent?: string[];
  keys: string[];
}

const KEYS = {
  theme: "mytools:theme",
  hotkey: "mytools:hotkey",
  tray: "mytools:trayEnabled",
  floatball: "mytools:floatballEnabled",
  pinned: "mytools:pinned",
  recent: "mytools:recent",
} as const;

function readString(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function readList(key: string): string[] | undefined {
  const raw = readString(key);
  if (!raw) return undefined;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return undefined;
    return parsed.filter((item): item is string => typeof item === "string");
  } catch {
    return undefined;
  }
}

export function readLegacyPreferences(): LegacyPreferences | null {
  const present: string[] = [];
  for (const key of Object.values(KEYS)) {
    if (readString(key) !== null) present.push(key);
  }
  if (present.length === 0) return null;

  const themeRaw = readString(KEYS.theme);
  const theme =
    themeRaw === "system" || themeRaw === "light" || themeRaw === "dark"
      ? themeRaw
      : undefined;
  const hotkeyRaw = readString(KEYS.hotkey)?.trim();
  const hotkey = hotkeyRaw ? hotkeyRaw : undefined;

  return {
    theme,
    hotkey,
    pinned: readList(KEYS.pinned),
    recent: readList(KEYS.recent),
    keys: present,
  };
}

/** Remove the legacy keys and any other `mytools:` prefixed host data. */
export function clearLegacyPreferences(keys: readonly string[]): void {
  try {
    for (const key of keys) localStorage.removeItem(key);
    const stale: string[] = [];
    for (let index = 0; index < localStorage.length; index += 1) {
      const key = localStorage.key(index);
      if (key && key.startsWith("mytools:")) stale.push(key);
    }
    for (const key of stale) localStorage.removeItem(key);
  } catch {
    /* storage unavailable */
  }
}
