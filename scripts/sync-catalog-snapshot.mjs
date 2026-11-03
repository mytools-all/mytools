import { writeFile } from "node:fs/promises";
import { CATALOG_URL, parseToolCatalog } from "../src/data/catalogModel.ts";

const response = await fetch(CATALOG_URL, {signal: AbortSignal.timeout(15000)});
if (!response.ok) throw Error(`Catalog HTTP ${response.status}`);
const body = await response.text();
if (Buffer.byteLength(body) > 2 * 1024 * 1024) throw Error("Catalog exceeds size limit");
const index = JSON.parse(body);
const entries = parseToolCatalog(index);
await writeFile(new URL("../src/data/catalogSnapshot.json", import.meta.url), JSON.stringify(index, null, 2) + "\n");
console.log(`Updated offline snapshot: ${entries.length} plugins`);
