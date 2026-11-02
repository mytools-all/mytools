/**
 * Generate a data-URI SVG "fallback" icon for a plugin name. Used when:
 *
 * - The plugin manifest has no `icon` (most tool packages don't ship one).
 * - The plugin webview is unavailable to resolve its own icon.
 *
 * The glyph is a tinted square with the package's first letter
 * (stripped of `@scope/`) centered. The hue is a stable function of
 * the package name so the same plugin always gets the same color.
 */
const iconCache = new Map<string, string>();

export function fallbackIconDataUri(name: string): string {
  const cached = iconCache.get(name);
  if (cached) return cached;
  const stripped = name.replace(/^@[^/]+\//, "");
  const initial = (stripped.trim().charAt(0) || "?").toUpperCase();
  let hash = 0;
  for (let i = 0; i < stripped.length; i++) {
    hash = (hash * 31 + stripped.charCodeAt(i)) >>> 0;
  }
  const hue = hash % 360;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40"><rect width="40" height="40" rx="8" fill="hsl(${hue},55%,52%)"/><text x="50%" y="55%" text-anchor="middle" dominant-baseline="middle" font-family="-apple-system,Segoe UI,Arial,sans-serif" font-size="20" font-weight="700" fill="#fff">${escapeXml(initial)}</text></svg>`;
  const uri = `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
  if (iconCache.size >= 512) iconCache.delete(iconCache.keys().next().value!);
  iconCache.set(name, uri);
  return uri;
}

function escapeXml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}