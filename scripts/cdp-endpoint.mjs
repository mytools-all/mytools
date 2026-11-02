import { readFile } from "node:fs/promises";
import path from "node:path";

export function parseActivePort(text) {
  const [portText, browserPath] = text.trim().split(/\r?\n/);
  const port = Number(portText);
  if (!/^\d+$/.test(portText ?? "") || !Number.isInteger(port) || port < 1 || port > 65535
    || !/^\/devtools\/browser\/[a-zA-Z0-9-]+$/.test(browserPath ?? "")) {
    throw new Error("Invalid or incomplete DevToolsActivePort file");
  }
  return { baseUrl: `http://127.0.0.1:${port}`, browserPath };
}

export async function discoverEndpoint({ port, profile, sessionFile }) {
  if (port !== undefined) {
    const number = Number(port);
    if (!Number.isInteger(number) || number < 1 || number > 65535) throw new Error("--port must be between 1 and 65535");
    return { baseUrl: `http://127.0.0.1:${number}` };
  }
  if (!profile) {
    try {
      const session = JSON.parse(await readFile(sessionFile, "utf8"));
      if (typeof session.profile !== "string" || !path.isAbsolute(session.profile)) throw new Error("invalid profile");
      profile = session.profile;
    } catch (error) {
      throw new Error(`Cannot read CDP session; run pnpm dev:debug or specify --port / --profile. ${error.message}`);
    }
  }
  const candidates = [path.join(profile, "EBWebView", "DevToolsActivePort"), path.join(profile, "DevToolsActivePort")];
  for (const candidate of candidates) {
    try { return parseActivePort(await readFile(candidate, "utf8")); }
    catch (error) { if (error.code !== "ENOENT") throw error; }
  }
  throw new Error("DevToolsActivePort not found; the debug instance has not started or has exited");
}
