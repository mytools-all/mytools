export interface PanelEntry {
  id: string;
  title: string;
  kind: "plugin" | "app" | "path" | "url";
  target: string;
  feature?: string;
}

export function validPanelEntry(value: unknown): value is PanelEntry {
  if (!value || typeof value !== "object") return false;
  const entry = value as Partial<PanelEntry>;
  if (typeof entry.id !== "string" || !entry.id || typeof entry.title !== "string" || !entry.title.trim() || typeof entry.target !== "string" || !entry.target.trim()) return false;
  if (entry.kind === "url") {
    try { return ["https:", "http:"].includes(new URL(entry.target).protocol); } catch { return false; }
  }
  if (entry.kind === "plugin") return /^[a-z0-9_-]+$/i.test(entry.target) && typeof entry.feature === "string" && Boolean(entry.feature);
  if (entry.kind === "path") return /^(?:[a-z]:[\\/]|\\\\[^\\]+\\[^\\]+)/i.test(entry.target);
  return entry.kind === "app";
}

export function movePanelEntry(entries: PanelEntry[], source: string, target: string): PanelEntry[] {
  const next = [...entries];
  const from = next.findIndex(x => x.id === source);
  const to = next.findIndex(x => x.id === target);
  if (from < 0 || to < 0 || from === to) return next;
  const item = next.splice(from, 1)[0];
  if (item) next.splice(to, 0, item);
  return next;
}
