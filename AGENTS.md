# AGENTS.md — MyTools 0.1.0

Windows 宿主，Tauri 2 + Vue 3 + Rust。只维护平台和能力，插件均为独立仓库与 npm 包，不在本仓库中保存源码或播种到二进制。

## 阅读顺序

docs/README.md、01-requirements.md、02-architecture.md、04-plugin-spec.md、07-testing.md、10-plugins-and-tools.md。

## 命令

- pnpm install --frozen-lockfile
- pnpm tauri dev；pnpm build；pnpm tauri build
- pnpm type-check；pnpm lint:frontend；pnpm test
- 在 src-tauri：cargo test --workspace；cargo clippy --workspace --all-targets -- -D warnings
- pnpm dev:install:debug 构建安装并启动 CDP；pnpm dev:debug 启动现有 release。
- node scripts/cdp-eval.mjs <expression> 自动发现实际端口。不要写死 9222；WebView2 使用触发调用写全局变量再轮询，避免 awaitPromise:true。

## 边界

- src-tauri/src/bootstrap.rs 装配；commands/ 只做 IPC；runtime/ 校验身份与执行能力；services/ 业务；infrastructure/ 系统适配。
- mytools-core 必须零 Tauri 依赖，能力 id 全小写。
- 前端 src/runtime/ 管理插件生命周期和授权；Pinia 状态；Vue script setup lang=ts；中文 UI。
- 不引入 Electron/Rubick 运行时兼容；参考交互不代表兼容。
- 默认插件目录是应用数据下的 plugins；用户配置和 MYTOOLS_PLUGINS_DIR 可覆盖。源码仓库不是已安装插件目录。

## 约定

- 除用户明确要求，不新增代码注释。不硬编码用户路径，不引入 UI 组件库。
- 不手工修改 pnpm-lock.yaml，不提交 node_modules、target、凭据、.npmrc、.env、运行数据。
- 不修改 icons/ 和 gen/ 生成文件。
- Rust 使用 thiserror / anyhow、tokio、tracing，产品路径避免 unwrap。
- Tauri 命令在 commands/，使用 domain_action 命名。

## Windows 构建

src-tauri/rust-toolchain.toml 锁定 GNU，不能切回 MSVC。build.rs 嵌入 Common-Controls manifest 的链接逻辑必须保留。WebView2Loader.dll 由 build.rs 复制，tauri.conf.json 的 bundle.resources 必须保持 map，目标为 exe 同级。

必须使用 pnpm tauri build --no-bundle 或 pnpm dev:install。直接 cargo build --release 不具备 Tauri CLI 的发布环境，可能生成加载 devUrl 的错误产物。

## 发布

宿主和插件版本起点为 0.1.0，项目 MIT，第三方许可不能覆盖。npm 插件使用 @mytools-all/<运行时身份名>，plugin.json.name 保持无作用域身份。索引仓库为 mytools-all/mytools-home。
