# MyTools

轻量级 Windows 插件工具箱，基于 Tauri 2、Vue 3 和 Rust。版本 0.1.0，MIT 许可。

宿主提供主页、插件管理、鼠标中键面板、托盘、悬浮球、权限管理和能力桥接。工具插件独立开发、安装和发布，不编译进宿主。

## 下载正式版

前往 [MyTools Releases](https://github.com/mytools-all/mytools/releases/latest)，下载 Windows x64 的 `MyTools_0.1.0_x64-setup.exe` 安装包。也提供包含主程序和 WebView2Loader.dll 的免安装运行 zip，解压后运行 mytools.exe；用户数据仍保存在 AppData，不是随目录移动的数据便携版。

校验文件为 SHA256SUMS.txt。第一次启动默认隐藏在托盘，可按 Alt+Space 或通过托盘“主页”打开。插件单独安装，不内置于安装包。完整版本记录见 [CHANGELOG.md](CHANGELOG.md)。

## 仓库

- [项目与插件索引](https://github.com/mytools-all/mytools-home)
- [插件 npm 索引](https://github.com/mytools-all/mytools-home/blob/main/plugins/database.json)
- npm 统一作用域：`@mytools-all`

## 从零编译与安装（Windows x64）

以下命令在 **PowerShell 7（pwsh）** 中执行，面向 Windows 10/11 x64。宿主可独立编译，不需要克隆索引仓库或任何插件仓库，也不需要 npm/GitHub 登录或 API Key。

### 1. 安装环境

| 工具                                                                        | 要求 / 用途                                                        |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| [Git](https://git-scm.com/downloads/win)                                     | 克隆源码                                                           |
| [Node.js](https://nodejs.org/)                                               | 24 或以上；安装后应有 npm                                          |
| pnpm                                                                        | `npm install --global pnpm@11`                                   |
| [PowerShell](https://github.com/PowerShell/PowerShell/releases)              | 7 或以上；安装脚本使用 pwsh，不是系统旧版 Windows PowerShell 5.1   |
| [Rustup](https://rustup.rs/)                                                 | 用于安装下方指定的 Windows GNU 工具链                              |
| [MSYS2](https://www.msys2.org/)                                              | 提供 MinGW-w64 GCC、windres 和链接工具                             |
| [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) | 安装 x64 Evergreen Runtime，负责显示界面；它不是需要单独打开的软件 |

项目当前验证的 GNU 环境使用 MSYS2 的 MINGW64。安装 MSYS2 后，在 **MSYS2 MINGW64** 终端执行：

```sh
pacman -Syu
pacman -S --needed mingw-w64-x86_64-toolchain
```

系统更新若要求关闭终端，重新打开同一终端，再完成包安装。不要混用 MINGW64、UCRT64 或 MSYS 的编译器路径。MINGW64 已被 MSYS2 标记为旧环境，本项目暂时保持已验证组合，迁移编译环境需另行验证。

回到 PowerShell 7，将实际 MinGW 目录加到当前终端 PATH。默认安装路径示例（自定义安装请改路径）：

```powershell
$env:PATH = "C:\msys64\mingw64\bin;$env:PATH"
git --version
node --version
pnpm --version
pwsh --version
rustup --version
gcc --version
windres --version
```

gcc、windres 均必须可找到；只有 Rust 而没有 windres 会在嵌入 Windows manifest 时构建失败。项目使用 `stable-x86_64-pc-windows-gnu`，不要按通用 Tauri 示例切回 MSVC。本机原有的 Visual Studio/MSVC 环境不能代替此处 GNU 工具。

### 2. 克隆与安装依赖

```powershell
git clone https://github.com/mytools-all/mytools.git
cd mytools
rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal --component rustfmt --component clippy
$env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-gnu"
pnpm install --frozen-lockfile
```

也可把仓库克隆为 mytools-home 工作区中的 app 目录。后续命令都在**宿主仓库根目录**运行；如果项目布局是 app，请先 cd app。

首次构建需要联网下载 npm 包、Rust crates 和 Tauri 打包工具；普通编译不依赖插件商店索引在线服务。若用了代理，确保 localhost/127.0.0.1 回环通信可用。

### 3. 开发运行

```powershell
pnpm tauri dev
```

此命令同时启动前端开发服务器和桌面宿主。只执行 pnpm dev 仅启动网页服务器，不能代替桌面宿主。MyTools 常驻系统托盘，初始主页可能隐藏；使用 **Alt+Space** 或托盘的“显示主页”唤出。快捷键冲突时先关闭占用软件，或从托盘进入设置修改。

### 4. 编译并安装

**生成普通安装包，交给自己或其他电脑安装：**

```powershell
pnpm tauri build --bundles nsis
Get-ChildItem ./src-tauri/target/release/bundle/nsis/*-setup.exe
```

双击该目录生成的 `*-setup.exe`，按安装向导完成安装。NSIS 默认安装到当前用户的 `%LOCALAPPDATA%/MyTools`；安装后可从开始菜单启动，随后用托盘或 Alt+Space 打开主页。安装包会包含必要的 WebView2 安装引导。

**只在开发机部署并直接启动：**

```powershell
pnpm dev:install
```

这会构建 release、关闭已有的 MyTools 进程并替换 `%LOCALAPPDATA%/MyTools` 下的程序，再启动它；执行前保存尚未提交的编辑内容。该命令不是完整安装向导，不负责创建开始菜单入口。需要调试时使用 pnpm dev:install:debug；只启动已有 release 调试构建可用 pnpm dev:debug。

**只编译、不安装：**

```powershell
pnpm tauri build --no-bundle
```

程序位于 src-tauri/target/release/mytools.exe，运行时同目录必须有 WebView2Loader.dll。GNU 构建会从 Rust 依赖生成/提取该 DLL，首次克隆无需手动准备或提交 DLL；复制 exe 给其他机器时必须一起复制 DLL，并确保目标安装了 WebView2 Runtime。

发布构建必须经 Tauri CLI，不能直接 cargo build --release；否则可能生成仍加载开发服务器地址的程序。需要 MSI 时可单独运行 pnpm tauri build --bundles msi，该路径还依赖 WiX/.NET 等打包条件，不作为上述 NSIS 安装路径的必需步骤。更多平台打包条件见 [Tauri Windows 安装包文档](https://v2.tauri.app/distribute/windows-installer/)。

### 5. 安装插件

宿主不捆绑工具插件。打开“插件商店”选择安装；也可在“开发者”页面使用 npm 包名或本地 zip/tgz 安装。商店分类与介绍读取 mytools-home/plugins/database.json，网络失败时保留缓存或使用内置快照。安装包仍需要能访问 npm。

- 我的日历：`@mytools-all/mytools-calendar@0.1.0`
- 开发示例：`@mytools-all/mytools-demo@0.1.1`

插件始终装到统一插件根目录，与 Git 克隆目录分离。卸载插件程序保留独立插件存储；整个应用卸载的数据行为见 [数据与存储](docs/05-data-and-storage.md)。

## 验证与 CI

```powershell
pnpm type-check
pnpm type-check:e2e
pnpm lint:frontend
pnpm test
cd src-tauri
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd ..
```

[GitHub CI](https://github.com/mytools-all/mytools/actions/workflows/ci.yml) 是 `.github/workflows/ci.yml` 定义的自动检查流程，推送 main 或提交 PR 时运行，不会发布 npm 或 GitHub Release：

| 检查          | 内容                                                              |
| ------------- | ----------------------------------------------------------------- |
| Frontend      | lint、类型检查、单元测试、覆盖率                                  |
| Rust          | clippy 与 workspace 测试                                          |
| Core coverage | Linux 下核心库覆盖率                                              |
| Build         | Windows GNU release 构建，上传 exe 与 WebView2Loader.dll 测试产物 |
| E2E smoke     | 启动真实 exe，检查主页、搜索计算与插件商店导航                    |

E2E 使用 tauri-driver 转发 WebDriver 请求，WebdriverIO（wdio）执行界面用例。Edge 浏览器和 WebView2 Runtime 可以是不同版本，**驱动必须匹配应用实际使用的 WebView2 Runtime**。setup-e2e-driver.ps1 通过应用旁的 Loader 查询该版本，再下载兼容驱动，不再按 Edge 浏览器版本下载。微软要求见 [WebView2 WebDriver 文档](https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/webdriver)。CI 的 e2e-diagnostics 附件保留驱动日志和失败截图。

手动执行 E2E：先构建，再安装 tauri-driver，在第一个 PowerShell 终端启动驱动：

```powershell
pnpm tauri build --no-bundle
cargo install tauri-driver --locked
pnpm test:e2e:driver
```

保持该终端运行，在第二个终端进入同一仓库，再执行 pnpm test:e2e。未设置 MYTOOLS_E2E_EXE 时默认使用仓库的 release exe。此测试会操作真实窗口；本地运行前保存工作并退出正在使用的 MyTools。CI 在独立 runner 上运行，不依赖任何已安装插件或私人数据。

调试端口自动发现，见 [AGENTS.md](AGENTS.md)，不要固定为 9222。完整 Tauri 平台前置条件可参考 [官方文档](https://v2.tauri.app/start/prerequisites/)，本项目的 GNU 约束以上述步骤为准。

## 文档与许可

应用基本信息由 [src/config/app.ts](src/config/app.ts) 统一提供：“关于”面板的版本、作者、简介、仓库、文档、反馈链接和许可读取 [package.json](package.json)，产品名读取 [tauri.conf.json](src-tauri/tauri.conf.json)。Tauri 应用版本也引用 package.json；修改后需要重新构建，不属于用户偏好设置。支持平台的展示文案在 app.ts 中维护。

新维护者或其他 AI 工具先读 [项目上下文](docs/PROJECT-CONTEXT.md) 与 [AGENTS.md](AGENTS.md)。插件作者使用 [插件开发指南](docs/plugin-development/README.md) 和 [Demo 案例](docs/plugin-development/demo-walkthrough.md)；[日历验证记录](docs/plugin-development/validation-calendar.md) 记录文档实作结果。架构和 API 总目录见 [docs](docs/README.md)。项目代码使用 [MIT](LICENSE)，第三方依赖保留各自的许可证。

离线目录快照无需手工维护。发布应用前可运行 `pnpm catalog:sync` 从统一索引更新并校验快照；普通构建不依赖此联网步骤。详情页可确认卸载已安装插件，插件存储数据保留。
