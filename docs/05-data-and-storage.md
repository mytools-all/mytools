# 05 · 数据与存储

> **状态**：设计稿（持续修订）· 相关：[04-plugin-spec](./04-plugin-spec.md) · [02-architecture](./02-architecture.md)

## 1. 存储布局（Layout B）

Windows 数据按「安装 / 数据 / 配置」三分，路径解析入口为 `infrastructure/paths.rs`：

| 位置 | 内容 | 备注 |
|---|---|---|
| `%LOCALAPPDATA%\MyTools\` | NSIS 安装目录 | 只放 `mytools.exe`、`WebView2Loader.dll` 等运行时文件 |
| `%LOCALAPPDATA%\MyTools\data\` | **本机数据** | `plugins/`、`mytools.db`、`plugin-files/`、`logs/` |
| `%APPDATA%\MyTools\` | **用户配置** | `settings.json`、`permissions.json`、`directory-grants.json`、`plugin-state.json`、`plugin-origins.json`、`plugin-store.json` |

- `%LOCALAPPDATA%\com.mytools.app\` 是 **WebView2 运行时 profile**（HTTP / GPU 缓存、iframe localStorage），由 WebView2 管理，不属于本布局、永不迁移。
- 已知取舍：NSIS 卸载脚本 `rmdir /r $INSTDIR` 会连带删除 `%LOCALAPPDATA%\MyTools\data\`（数据随安装目录同生共死，卸载即清）；`%APPDATA%\MyTools\` 配置保留。若未来要求卸载后保留数据，需把 data 根移出 `$INSTDIR`（如 `%LOCALAPPDATA%\MyToolsData`）。
- 旧标识目录 `%APPDATA%\com.mytools.app\` 由启动时的一次性迁移搬空（见 §1.1）。

### 1.1 一次性迁移

启动最前执行 `commands/migration.rs`：

- **v2（布局迁移）**：把旧 `%APPDATA%\com.mytools.app\` 下的配置文件搬到 `%APPDATA%\MyTools\`，数据项（`plugins/`、`mytools.db`、`plugin-files/`、`logs/`）搬到 data 根；目标已存在则保留目标、不动源；跨盘 rename 失败回退 copy+校验+删源；搬空后删除旧目录。成功写入 `%APPDATA%\MyTools\layout-v2.json` 标记，之后启动直接跳过；失败不写标记，下次启动重试。
- **v3（生态清退）**：删除 `plugin-origins.json` 里标记为生态来源的插件目录与记录；`rubick-store.json` 改名为 `plugin-store.json`；删除已废弃的商店缓存目录。标记文件 `%APPDATA%\MyTools\layout-v3.json`。

## 2. SQLite

### 2.1 `clipboard` — 剪贴板历史

```sql
CREATE TABLE clipboard (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    content     TEXT    NOT NULL,
    pinned      INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_clipboard_created_at ON clipboard(created_at DESC);
```

约束

- 上限 500 条，超出按 `created_at DESC` 淘汰非置顶项
- 同内容不新增，只 bump `created_at`
- 清空只删非置顶项
- 建表幂等（打开连接时 `CREATE TABLE IF NOT EXISTS`）

### 2.2 不落库的状态

- **插件索引**：以插件目录为真源，禁用状态单独存 `plugin-state.json`；不建表，避免与文件状态双写。
- **应用索引**：内存快照 + TTL 惰性刷新，不落盘。

## 3. 剪贴板采集

| 项 | 规约 |
|---|---|
| 频率 | 200ms 轮询 |
| 启动 | 应用 setup 阶段启动后台任务 |
| 去重 | 缓存上次内容，相同则跳过 |
| 通知 | 新条目 → 落库 + 广播变更事件 |
| 前端 | 订阅变更事件自动刷新 |

## 4. 配置

`settings.json` 为**手写 JSON 文档**（非 KV 插件），存于配置根（`%APPDATA%\MyTools\settings.json`），字段按需扩展，核心项：

| 字段 | 说明 |
|---|---|
| `pluginRoot` | 自定义插件目录；缺省 = 平台默认 |
| `theme` | `system` / `light` / `dark` |
| `hotkey` | 全局唤起快捷键 |
| `pinnedPlugins` / `recentPlugins` | 固定插件 / 最近使用 |
| `trayEnabled` / `floatballEnabled` | 托盘 / 悬浮球开关 |
| `floatballX` / `floatballY` | 悬浮球位置 |
| `mainX` / `mainY` / `mainWidth` / `mainHeight` | 主窗口位置与尺寸 |
| `detachedWindowWidth` / `detachedWindowHeight` | 分离窗口尺寸 |

**迁移规则**：已废弃的字段（如旧的插件商店数据源、显示模式）加载时忽略，下次保存即消失。

所有用户偏好统一存 `settings.json`。旧版本使用的 `mytools:*` localStorage 键只在启动时做**一次性迁移**：写入 settings.json 成功后清空宿主 localStorage，之后不再读写。浏览器预览（无 Tauri 后端）只使用内存，不持久化。

## 5. 插件相关数据

| 数据 | 位置 | 生命周期 |
|---|---|---|
| 插件文件 | `<data>/plugins/<safeName>/`（可自定义根；优先级：环境变量 > 用户设置 > 平台默认） | 安装/卸载时增删 |
| 启用状态 | `plugin-state.json` | 持久化 |
| 插件私有存储 | 独立 KV 文件 `plugin-store.json`，键按插件 id 分区 | 随插件卸载清理 |
| 能力授权记录 | `permissions.json`（插件 + 能力粒度） | 持久化，可在设置中撤销 |
| 目录授权记录 | `directory-grants.json`（插件 + 目录粒度） | 持久化，可在设置中撤销 |
| 插件文件能力沙箱 | `<data>/plugin-files/<scope>/` | 沙箱路径，拒绝绝对路径与穿越 |

**安全**：插件私有存储按插件 id 隔离，插件只能访问自己的分区。

> 仓库的 `独立插件仓库` 只是工具包**源码**（不进二进制、不产生运行时数据）；装好后插件文件在
> `<data>/plugins/<name>/`，插件的私有数据在 `<data>/plugin-files/<name>/`。

## 6. 错误处理

- 存储层错误用 `thiserror` 定义结构化错误类型；命令层转换为统一错误码返回前端。
- 前端按错误码分支处理（见 [04-plugin-spec](./04-plugin-spec.md) §4.3）。

## 7. 同步（规划）

多端同步不在当前范围。若实现：配置始终同步；剪贴板历史（最近 100 条）可选；插件目录与索引不同步（按机器独立）。
