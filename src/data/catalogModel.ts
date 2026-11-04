export type ToolStatus = "builtin" | "ready" | "planned";
import catalogDefaults from "../config/catalog.json" with { type: "json" };
export const CATALOG_URL = catalogDefaults.pluginList;

export interface ToolReference {
  package: string;
  label: string;
  url: string;
}

export interface ToolEntry {
  name: string;
  title: string;
  description: string;
  category: string;
  version: string;
  status: ToolStatus;
  capabilities: string[];
  features: string[];
  keywords: string[];
  notes: string[];
  npmSpec?: string;
  npm: string;
  author: string;
  license: string;
  repository: string;
  homepage?: string;
  icon?: string;
  replaces: string[];
  reference?: ToolReference;
}

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw Error("插件索引格式无效");
  return value as Record<string, unknown>;
}

function text(value: unknown, max = 5000): string {
  if (typeof value !== "string" || !value.trim() || value.length > max) throw Error("插件索引缺少有效字段");
  return value.trim();
}

function strings(value: unknown): string[] {
  if (value === undefined) return [];
  if (!Array.isArray(value) || value.length > 256) throw Error("插件索引列表无效");
  return value.map(item => text(item));
}

function link(value: unknown): string {
  const url = new URL(text(value));
  if (url.protocol !== "https:" || url.username || url.password) throw Error("插件链接必须使用 HTTPS");
  return url.href;
}

export function parseToolCatalog(value: unknown): ToolEntry[] {
  const index = record(value);
  if (index.schemaVersion !== 2 || !Array.isArray(index.plugins) || !index.plugins.length || index.plugins.length > 5000) throw Error("不支持的插件索引版本");
  const seen = new Set<string>();
  return index.plugins.map(raw => {
    const p = record(raw);
    const name = text(p.name, 160);
    const npm = text(p.npm, 200);
    const version = text(p.version, 100);
    const packageName = /^(?:@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*$/;
    if (!packageName.test(name) || !packageName.test(npm) || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version) || typeof p.published !== "boolean" || seen.has(name)) throw Error("插件身份、版本或发布状态无效");
    seen.add(name);
    const reference = p.reference === undefined ? undefined : record(p.reference);
    return {
      name, npm, version,
      title: text(p.title, 200), description: text(p.description), category: text(p.category, 80),
      author: text(p.author, 200), license: text(p.license, 100),
      status: p.published ? "ready" : "planned",
      npmSpec: p.published ? `${npm}@${version}` : undefined,
      capabilities: strings(p.capabilities), features: strings(p.features),
      keywords: strings(p.keywords), notes: strings(p.notes), replaces: strings(p.replaces),
      repository: link(p.repository), homepage: p.homepage === undefined ? undefined : link(p.homepage),
      icon: p.icon === undefined ? undefined : link(p.icon),
      reference: reference ? {package: text(reference.package), label: text(reference.label), url: link(reference.url)} : undefined,
    };
  });
}
