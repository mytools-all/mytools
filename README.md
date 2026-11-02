# MyTools

轻量级 Windows 插件工具箱，基于 Tauri 2、Vue 3 和 Rust。版本 0.1.0，MIT 许可。

宿主提供启动器、插件管理、鼠标中键面板、托盘、悬浮球、权限管理和能力桥接。工具插件独立开发、安装和发布，不编译进宿主。

## 仓库

- [项目与插件索引](https://github.com/mytools-all/mytools-home)
- [插件 npm 索引](https://github.com/mytools-all/mytools-home/blob/main/plugins/database.json)
- npm 统一作用域：`@mytools-all`

## 开发

要求 Windows、Node 24+、pnpm 11+、Rust 和 WebView2 Runtime。本机开发配置锁定 GNU Rust 工具链。

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

本仓库可独立构建，无需克隆任何插件仓库。插件商店使用随应用维护的目录快照，安装时下载 npm 包。集中索引保存在 mytools-home，当前尚未动态读取远程 JSON。开发者也可安装本地目录、zip 或 tgz。

## 验证与打包

```powershell
pnpm type-check
pnpm lint:frontend
pnpm test
cd src-tauri
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd ..
pnpm tauri build
```

日常安装验证用 `pnpm dev:install:debug`，已有构建可用 `pnpm dev:debug`。调试端口自动发现，见 [AGENTS.md](AGENTS.md)。必须通过 Tauri CLI 构建发布版，不能用直接 cargo build 替代。

## 文档与许可

架构和插件 API 见 [docs](docs/README.md)。项目代码使用 [MIT](LICENSE)，第三方依赖保留各自的许可证。
