# 07 · 测试策略

> **状态**：设计稿（持续修订）· 相关：[02-architecture](./02-architecture.md) · [06-roadmap](./06-roadmap.md)

## 1. 测试分层

```
        ┌──────────────┐
        │  E2E / 手动  │  真实场景、关键路径
        ├──────────────┤
        │   集成测试   │  插件加载、IPC 端到端、生成物一致性
        ├──────────────┤
        │ 组件 / Store │  Vue 组件、Pinia
        ├──────────────┤
        │   单元测试   │  纯函数、算法、解析器、注册表
        └──────────────┘
```

| 层 | 范围 | 工具 |
|---|---|---|
| Rust 单元 | 清单解析、能力注册表、权限判定、SQLite CRUD、模糊匹配、路径安全 | `cargo test` |
| Rust 集成 | 命令、插件扫描/安装/卸载、资源服务与注入、迁移 | `cargo test --workspace` |
| 前端单元 | 组件、Pinia store、composables、纯函数 | Vitest + Vue Test Utils + happy-dom |
| E2E | 唤起 → 搜索 → 打开插件 → 插件中心 | tauri-driver + WebdriverIO |
| 工具包冒烟 | 每个 `tools/<name>` 装上后打开、核心交互走通 | 手动 + 工具包自带检查 |
| 手动冒烟 | 安装/卸载、剪贴板、多显示器、失焦隐藏 | 每版本发布前 |

## 2. 各架构层的测试职责

| 层 | 必测项 |
|---|---|
| **L1 基础层** | 能力注册表（注册/查找/调用/未知能力）、权限策略四种判定、清单规范解析与向后兼容、资源服务（token 前缀、目录穿越、Content-Type）、错误总线（采集/去噪/上限）、目录授权（授予/前缀边界/撤销/大小写不敏感） |
| **L2 应用层** | 官方工具视图交互、插件中心（安装/启用/卸载/打开目录）、来源标记展示、目录授权设置页、桥接协议（请求/响应/错误信封、非法消息丢弃） |

## 3. 目录约定

```
src/                       # 前端
├── App.test.ts            # 顶层组件测试（与 App.vue 同目录）
├── components/*.test.ts   # 组件单测
├── stores/*.test.ts       # Pinia store 单测
└── __tests__/             # 跨模块辅助测试

src-tauri/
├── src/commands/*.rs      # Tauri IPC 与桌面交互适配
├── src/services/          # 业务规则、索引、持久化（含单测）
├── src/infrastructure/    # Windows 图标、路径、后台采集（含单测）
├── src/architecture_tests.rs # 依赖方向与 IPC 声明边界
└── crates/mytools-core/   # 纯 Rust，零 Tauri 依赖
    └── src/*.rs
```

> 拆 workspace 的目的：让核心 crate 的测试在任何环境跑得通（不依赖 Tauri runtime / WebView2）。

业务服务、运行时和平台设施不得引用 `commands`，也不得声明 `#[tauri::command]`；架构边界测试随后端单测执行，新增模块同样受约束。

## 4. E2E

```bash
# 启动 tauri-driver（自动配 PATH 找 msedgedriver，注入 WEBVIEW2 远程调试参数）
pnpm test:e2e:driver

# 另一终端：跑 wdio 冒烟（依赖 driver 已起 + MYTOOLS_E2E_EXE 指到可执行文件）
pnpm test:e2e

# 清理
Get-Process tauri-driver,msedgedriver -ErrorAction SilentlyContinue | Stop-Process -Force
```

冒烟场景：窗口标题 → 搜索框存在 → 输入查询出结果 → 打开插件中心 → Escape 关闭。

## 5. CI

`.github/workflows/ci.yml` 五段式：

| Job | 运行环境 | 内容 |
|---|---|---|
| **frontend** | windows-2022 | `pnpm install` → `lint:frontend` → `type-check` → `test:coverage`（含门禁） |
| **rust** | windows-2022（GNU stable） | `lint:backend`（clippy `-D warnings`）→ `cargo test --workspace` |
| **backend-coverage** | ubuntu-22.04 | `cargo-llvm-cov --workspace --fail-under-lines 50` |
| **build** | windows-2022 | `pnpm tauri build --no-bundle` → 上传产物 |
| **e2e** | windows-2022 | 下载产物 → 装 msedgedriver → 起 tauri-driver → `pnpm test:e2e` |

并发控制：`concurrency: { group: ci-${{ github.ref }}, cancel-in-progress: true }`。

## 6. 覆盖率门禁

| 阶段 | Rust | 前端 |
|---|---|---|
| v0.3 | 行 ≥ 60% | 行 ≥ 50% |

后端用 `cargo-llvm-cov`，前端用 `@vitest/coverage-v8`。本机 Windows GNU 限制见 §8。

## 7. 文档-代码同步

| 里程碑 | 必须同步的文档 |
|---|---|
| M1 能力注册表 | [04-plugin-spec](./04-plugin-spec.md) §4/§7、[02-architecture](./02-architecture.md) §4 |
| M2 清单 v1 | [04-plugin-spec](./04-plugin-spec.md) §3/§5 |
| M5 能力开放 | [04-plugin-spec](./04-plugin-spec.md) §4.2、[02-architecture](./02-architecture.md) §5 |
| M7 工具包体系 | [10-plugins-and-tools](./10-plugins-and-tools.md)、[06-roadmap](./06-roadmap.md) §1 状态列 |

规则同 [06-roadmap](./06-roadmap.md) §5。

## 8. 本地运行限制

| 主题 | 现象 | 处置 |
|---|---|---|
| **后端覆盖率** | Windows + GNU stable 报 `E0463: can't find crate for 'profiler_builtins'` | 本机不跑后端覆盖率，**CI 在 ubuntu-22.04 跑** |
| **E2E 本地冒烟** | Edge Dev Channel 与 msedgedriver Stable 版本不匹配 → session 建立失败且无输出 | 本机跳过；CI 用 windows-2022 + Edge Stable。必须本地跑时把 Edge 切 Stable 并装匹配 msedgedriver |

## 9. 手动冒烟 Checklist

```
[ ] 干净环境可安装并首次启动
[ ] 主面板可被全局快捷键唤起，失焦自动隐藏
[ ] 搜索能命中内置工具与系统应用
[ ] 内置工具均可完成预期功能
[ ] 剪贴板历史可记录并在重启后保留
[ ] 多显示器下主面板定位到当前显示器
[ ] 插件中心可安装（目录/归档/包管理器）、启用停用、卸载
[ ] 从 `独立插件仓库` 安装一个工具包后可正常运行（不白屏）
[ ] 工具包首次调用写类能力会弹授权确认，确认后可正常工作
[ ] 工具包访问真实文件需选目录授权，越界路径被拒
[ ] 插件崩溃时宿主显示错误面板且不卡死
[ ] 托盘/悬浮球开关即时生效
```


## 10. CDP 实机诊断

调试端口由 WebView2 自动分配，助手读取独立配置目录中的 `DevToolsActivePort`，并核对浏览器实例标识；不依赖固定端口。只为显式启动的诊断实例开启 CDP，普通应用启动无需开放调试端口。

```powershell
pnpm dev:install:debug

node scripts/cdp-eval.mjs 'window.__cdp = document.title'
node scripts/cdp-eval.mjs 'window.__cdp = window.mytools.getEnv()' --frame mytools-hello
```

已有构建产物时用 `pnpm dev:debug` 直接启动仓库 release EXE；也可执行 `pwsh -NoProfile -File scripts/start-cdp.ps1 -ExePath <exe路径>`。
启动器输出实际端口并保存会话到 `src-tauri/target/cdp-session.json`，助手默认读取该会话。
多实例时以 `--profile <配置目录>` 指定实例；连接外部已启动的调试实例时以 `--port <端口>` 显式指定。
`--window <窗口label>` 选择主窗口或分离窗口，默认 `main`，不依赖当前路由是否是启动器。

脚本测试：`pnpm test:debug`。调试命令在普通权限终端执行；宿主提升权限时，WebView2 可能忽略环境变量覆盖。
Windows 排除端口与占用端口都可能造成固定端口不可绑定，应优先采用自动分配，而非修改整机端口排除规则。
