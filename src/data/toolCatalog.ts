import snapshot from "./catalogSnapshot.json";
import { parseToolCatalog } from "./catalogModel";

export * from "./catalogModel";
export const TOOL_CATALOG = parseToolCatalog(snapshot);
export const TOOL_CATEGORIES = [...new Set(TOOL_CATALOG.map(entry => entry.category))];
