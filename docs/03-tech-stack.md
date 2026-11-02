# 03 · 技术栈选型

> **状态**：设计稿（持续修订）· 相关：[02-architecture](./02-architecture.md)

## 1. 选型一览

| 层 | 技术 | 版本 | 理由 |
|---|---|---|---|
| 前端框架 | Vue 3 + Composition API | 3.5+ | `<script setup>` 简洁，Vite 启动快 |
| 构建 | Vite | 5+ | 冷启动 < 1s，HMR 快 |
| 类型 | TypeScript | 5+ | 插件协议与 IPC 需要强类型契约 |
| 状态 | Pinia | 2+ | 官方推荐，DevTools 友好 |
| 路由 | Vue Router | 4+ | 启动器 / 插件 / 设置多视图 |
| 工具集 | @vueuse/core | 12+ | 组合式工具，少造轮子 |
| 后端 | Tauri | 2.x | 见 §2 |
| 语言 | Rust | 1.77+ | Tauri 2 要求 |
| 存储 | SQLite（rusqlite bundled） | 0.32 | 单文件、零部署、免系统库 |
| 异步 | tokio | 1.x | Tauri 2 默认 runtime |
| 测试 | Vitest + happy-dom / cargo test | — | 前端 DOM 测试 + Rust 单测 |

## 2. 为什么是 Tauri 2

| 对比项 | Tauri 2 | Electron |
|---|---|---|
| 渲染 | 系统 WebView2 / WKWebView | 内嵌 Chromium |
| 后端 | Rust | Node.js |
| 安装包 | ~5-10MB | ~100MB+ |
| 内存 | 20-60MB | 100-300MB |
| 启动 | < 500ms | 1-3s |
| 安全 | 默认 sandbox + capability 声明 | 需手动配置 |

目标形态是"启动器 + 插件化工具箱"，工具以插件包形态交付；Tauri 2 兼顾轻量与系统 WebView 复用，是当前唯一成熟解。

## 3. 依赖清单

### 3.1 运行时

| 类别 | 依赖 | 用途 |
|---|---|---|
| 框架 | `tauri`（features `tray-icon`） | 主框架、系统托盘 |
| 能力插件 | `tauri-plugin-{opener,global-shortcut,clipboard-manager,shell,store,dialog,fs}` | 官方能力插件（对应 `capabilities/default.json` 权限声明） |
| 序列化 | `serde` / `serde_json` | 清单、IPC 载荷 |
| 异步 | `tokio` | 异步 runtime |
| 错误 | `thiserror` / `anyhow` | 库级结构化错误 / 应用层错误 |
| 日志 | `tracing` / `tracing-subscriber` / `tracing-appender` | 结构化日志 + rolling file |
| 搜索 | `fuzzy-matcher` | 模糊匹配（Skim 算法） |
| 遍历 | `walkdir` | 目录遍历 |
| 监听 | `notify` + `notify-debouncer-mini` | 插件目录热更新 |
| 网络 | `ureq` | HTTP 客户端（包管理器、插件目录） |
| 本地服务 | `tiny_http` | 回环静态服务（把插件目录以 http 暴露给 iframe） |
| 进程 | `sysinfo` | 进程信息（健康检查） |
| 图像 | `image`（仅 png） | 位图 → PNG data URI |
| 平台 | `windows-sys` / `windows` | Windows API：Shell（图标/快捷方式解析）、GDI、COM、文件系统 |

### 3.2 核心 crate（零 Tauri 依赖）

| 依赖 | 用途 |
|---|---|
| `serde` / `serde_json` / `thiserror` | 清单解析与错误 |
| `rusqlite`（bundled） | 剪贴板持久化 |
| `chrono` | 时间戳 |
| `fuzzy-matcher` | 搜索打分 |
| `meval` | 计算器表达式求值 |
| `walkdir` | 扫描 |
| `zip` / `tar` / `flate2` | 插件归档安装 |

> **硬约束**：核心 crate 必须保持零 Tauri 依赖。一旦引入 `tauri*`，`--workspace` 会把 webview2/comctl32 链拉进 core 测试二进制，导致缺 Common-Controls v6 manifest 而崩溃。

### 3.3 明确不引入

| 方案 | 原因 |
|---|---|
| UI 组件库 | 主面板交互极简，手写 CSS 更小更可控 |
| Tailwind | CSS 变量 + scoped style 已够用 |
| `reqwest` / `sqlx` | `ureq` / `rusqlite` 已满足当前需求，体量与异步模型更契合 |

## 4. 工具链

| 工具 | 版本 | 用途 |
|---|---|---|
| Node.js | 24+ | 前端构建 |
| pnpm | 11+ | 包管理（`pnpm-workspace.yaml` 必填） |
| Rust / cargo / rustup | 1.77+ | 后端 |
| Tauri CLI | 2.x | `pnpm tauri dev/build` |
| Vitest + happy-dom | 2+ / 15+ | 前端测试 |
| tauri-driver + WebdriverIO | — | E2E |

## 5. 平台约束（Windows）

1. **WebView2Loader.dll 必须与 exe 同级**。GNU 工具链下 `webview2-com` 动态链接该 DLL；`build.rs` 在构建时复制到资源目录，打包时用 **map 形式**的 `bundle.resources` 放到安装根目录（数组形式会落进 `resources\` 子目录，运行时报「找不到 WebView2Loader.dll」）。
2. **核心 crate 不得引入 `tauri*`**（见 §3.2）。
3. **MinGW 导出序号**：静态库导出符号过多会溢出，需限制 `crate-type`（见 `src-tauri/Cargo.toml` 注释）。

## 6. 升级策略

| 依赖 | 策略 |
|---|---|
| Tauri | 跟随官方 GA，不随意升 minor（permission 系统易 breaking） |
| Vue / Pinia | minor 每季度评估，major 必走 PR |
| Rust crates | 跟随上游，CI 拦截 breaking |
| Node | 跟随 LTS |
