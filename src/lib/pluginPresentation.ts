import { TOOL_CATALOG } from "@/data/toolCatalog";
import type { PluginManifest } from "@/stores/plugins";

const titles = new Map(TOOL_CATALOG.map((entry) => [entry.name, entry.title]));

export function pluginDisplayName(manifest: Pick<PluginManifest, "name" | "pluginName">): string {
  return manifest.pluginName?.trim() || titles.get(manifest.name) || manifest.name;
}
