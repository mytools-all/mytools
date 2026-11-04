# MyTools 项目上下文

状态：实现说明，核对日期 2026-11-04。本文是跨 AI 工具的起点；不包含账号、机器私有路径、令牌或用户数据。

## 项目与仓库

MyTools 是 Windows 插件工具箱，宿主 Tauri 2 + Vue 3 / TypeScript + Pinia + Rust。主页搜索应用和插件，插件中心负责商店、安装、开发与偏好设置；鼠标中键“我的面板”、托盘和悬浮球提供入口。插件可以嵌入主页或分离为窗口。

本地父目录包含 mytools-home 的 README、AGENTS 和 plugins/database.json；app 是 mytools 仓库，各 plugins/<name> 是独立仓库，不是子模块。单独克隆宿主即可构建，克隆索引不会自动下载插件。仓库链接以父目录 README 为准。

## 实际架构

| 路径（相对宿主） | 职责 |
|---|---|
| src/App.vue、composables/useAppShell.ts | 窗口壳、初始化和桌面事件 |
| src/runtime、composables/pluginBridge.ts | 插件生命周期、消息、错误、授权队列 |
| src/stores、views、components | 界面与状态 |
| src/views/PluginManagerView.vue | 插件中心导航、详情抽屉与宿主窗口布局 |
| src/views/plugin-center | 商店、已安装、开发者、偏好设置四个独立页面与公共样式；KeepAlive 保留切页状态 |
| src/config/app.ts | 应用基本信息统一读取入口，供“关于”等界面使用，与用户偏好 store 分离 |
| src-tauri/src/bootstrap.rs | 装配 |
| src-tauri/src/commands | Tauri IPC 和窗口适配 |
| src-tauri/src/runtime | 会话、身份、能力调度、启动上下文 |
| src-tauri/src/services | 插件、应用、搜索、设置和能力业务 |
| src-tauri/src/infrastructure | Windows、路径、网络、图标等适配 |
| src-tauri/crates/mytools-core | 不依赖 Tauri 的清单、权限和规则 |

插件目录由本地 HTTP 服务提供，宿主注入 window.mytools 桥接，插件始终处于 iframe 中。消息绑定窗口、来源、插件身份及会话，再进入能力注册表与授权策略。独立窗口也不等于插件可以调用 Tauri。Rubick/uTools 只作为功能和交互参考，不兼容其 Electron API。

商店信息维护在 mytools-home/plugins/database.json（schemaVersion 2）；宿主异步读取远程索引，失败使用有效缓存或 src/data/catalogSnapshot.json。快照用 pnpm catalog:sync 生成。npm 分发安装包；plugin.json 决定运行时身份与实际能力。索引介绍不会授予权限。

## 运行与验证

Windows、Node 24+、pnpm 11+、WebView2 Runtime，Rust 使用仓库锁定的 GNU 工具链。在 app 中执行：

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
pnpm type-check
pnpm lint:frontend
pnpm test
```

后端在 app/src-tauri 执行 cargo test --workspace 与 cargo clippy --workspace --all-targets -- -D warnings。发布构建必须用 pnpm tauri build，或 pnpm tauri build --no-bundle；不要直接 cargo build --release。保留 GNU Common-Controls manifest 与 WebView2Loader.dll 打包逻辑。

pnpm dev:install:debug 会重建、替换并重启本机安装版本，验收前保存正在编辑的内容。只启动现有构建用 pnpm dev:debug。CDP 地址读取 src-tauri/target/cdp-session.json，不写死 9222；重启后会话可能过期。scripts/cdp-eval.mjs 自动发现端点，异步表达式通过写 window.__cdp 再轮询返回，不使用 awaitPromise:true。

## 数据与兼容边界

- 默认插件根：%LOCALAPPDATA%/MyTools/data/plugins，可由设置或 MYTOOLS_PLUGINS_DIR 覆盖；源码目录不是安装目录。
- 配置与 storage.*：%APPDATA%/MyTools。插件持久化使用 storage.*，不用回环 HTTP 源的 localStorage（端口/配置变化会更换源）。
- 文件沙箱、数据库和日志：%LOCALAPPDATA%/MyTools/data。插件卸载保留独立存储；整个应用卸载的数据行为见 05-data-and-storage.md。
- 面板仅采集资源管理器选中文件与剪贴板文字/图片，context.read 按指定插件和窗口一次性交付。不是任意软件选区读取器。
- 窗口显隐、托盘“主页”、分离窗口任务栏身份和图标由宿主统一维护。当前图标来自 design/mytools-icon-v1.png；ICO 资源、Windows 窗口图标和任务栏身份均需一致。
- 插件 API 的实际 iframe 错误是 Error(message)，不保证 code/consentRequired；授权由宿主完成。详见插件指南，不把目标接口当成已实现接口。
- storage.* 不传 namespace，使用宿主默认插件作用域。当前后端仍接受 namespace 覆盖参数，不应据此宣称不可信插件存储具有完整安全隔离；这属于后续安全加固事项。

## 修改与交接

应用基本信息维护位置：package.json 中的 version、description、author、repository、homepage、bugs、license；产品名和应用标识在 src-tauri/tauri.conf.json。Tauri 的 version 引用 ../package.json，因此界面与安装包共用应用版本号；src/config/app.ts 组装只读 APP_INFO，页面不重复填写这些值。支持平台的展示文案也在该入口维护。Cargo.toml 的 crate 版本仍是 Rust 包元数据，发布调整时独立检查，不作为“关于”页面版本来源。修改基本信息后需重新构建应用；它不是用户运行时设置。

架构详细说明见 02-architecture.md，完整能力目录见 04-plugin-spec.md，面板见 13-launch-context.md。面向新插件作者只需插件开发指南和 demo，不需要读宿主实现。新能力需先定义参数、返回、权限和测试；普通插件功能不改宿主。

每次交接更新本次改动、运行过的检查、未解决问题和发布状态，避免写机器临时 PID/端口或依赖聊天记录。已有未提交修改应先用 git status / git diff 确认，不能整体覆盖。

离线整理尚未推送的本地索引时，可在宿主目录执行 `pnpm catalog:sync --file ../plugins/database.json`，与远程同步使用相同的字段校验；无参数仍读取远程索引。

## 插件目录主仓库与镜像

默认地址统一维护在 `src/config/catalog.json` 的 `pluginList`、`pluginListMirror`，由 `APP_INFO` 暴露，Rust 同时嵌入同一文件。GitHub 主仓库与 Gitee 镜像均指向 mytools-all/mytools-home 的 main/plugins/database.json。

偏好设置 → 插件顶部可编辑、恢复默认地址并保存刷新；用户覆盖值持久化在 settings.json 的 catalogSources 中，“关于”末尾展示当前配置的两行地址。每次目录刷新先访问主仓库，请求失败或目录校验失败后尝试镜像，成功地址显示为当前使用仓库；后续刷新仍优先主仓库。两者失败保留已有缓存或内置快照，不安装来自无效索引的数据。仅支持不携带账号密码的 HTTPS 地址；修改目录源不会更改 npm 下载源或插件安装位置。

## 插件更新与本地发现

商店、详情和已安装页比较索引版本与已安装清单版本，按 SemVer 排序，仅对已发布、可安装且更高的版本显示“可更新”。相同版本、较旧版本、无效版本、published:false 均不提供 npm 更新。已安装页提供“检查更新 / 重新扫描”、可更新筛选和顺序批量更新；首次进入也扫描安装目录，保证新放入的独立插件能够出现。本地源码插件必须安装到实际插件根目录，不能只改工作区。

plugin_update_npm 下载固定版本后，由 core 更新事务在插件根内暂存并校验插件身份、目标版本、当前版本与入口路径，再通过目录替换更新。普通激活失败恢复旧目录；恢复失败则保留旧目录并返回恢复路径。更新不卸载插件、不删除 storage 或文件沙箱，保留启停状态；运行时已有版本变化监听会重建插件实例。安装/更新/卸载及设置目录操作串行。批量更新继续处理后续插件并报告失败项。源码与安装包更新不等于 npm 发布；只有发布新版并更新 database.json 后，其他用户才会获得新版提示。

本次本地验证：309 项前端测试、163 项宿主 Rust 测试、125 项 core 测试通过（4 项原有忽略），类型检查、前端 lint（0 错误，13 项既有警告）、clippy 和 Tauri 发布构建通过。已部署本机，实际 WebView2 验证主页搜索打开打卡、已安装页发现打卡/更新入口、打卡设置侧栏和日历标签管理窗口；未操作用户记录。日历 36 项、打卡 17 项独立测试通过。真实 npm 新版本更新、失败场景的原生 UI 端到端操作仍未验收；身份/版本拒绝和失败恢复已由原生文件系统单元测试覆盖。未推送或发布。

Windows 发布通过宿主目标依赖 xz2/static 强制静态链接 liblzma，保留 ZIP 原有格式支持。scripts/check-runtime-deps.mjs 递归读取 EXE/DLL 的 PE 导入表，已知 Windows 系统 DLL 以外的依赖必须与程序同目录；CI 在上传运行包前执行此检查。THIRD_PARTY_NOTICES.txt 和 LICENSE 随安装包及免安装包交付。发布验收须在不含 MSYS2/MinGW/Git/编译器目录的 PATH 下运行，避免开发环境 DLL 掩盖缺失依赖。
