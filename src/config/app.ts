import catalogDefaults from "./catalog.json";
import packageInfo from "../../package.json";
import tauriConfig from "../../src-tauri/tauri.conf.json";

export const APP_INFO = Object.freeze({
  name: tauriConfig.productName,
  identifier: tauriConfig.identifier,
  version: packageInfo.version,
  description: packageInfo.description,
  author: packageInfo.author.name,
  authorUrl: packageInfo.author.url,
  repository: packageInfo.repository.url,
  homepage: packageInfo.homepage,
  issues: packageInfo.bugs.url,
  license: packageInfo.license,
  platform: "Windows",
  ...catalogDefaults,
});
