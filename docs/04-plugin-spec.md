# 04 · 插件规范

> **状态**：设计稿（持续修订）· 相关：[02-architecture](./02-architecture.md) · [05-data-and-storage](./05-data-and-storage.md)
> **本文回答**：一个插件长什么样？怎么声明所需能力？怎么被授权、怎么被运行、怎么被打包安装？

## 1. 设计原则

1. **向后兼容优先**：所有新增字段可选；缺字段的老插件必须照常运行。
2. **声明式**：插件**声明**需要什么能力，宿主**决定**是否给；插件不自行判断权限。
3. **单一规范面**：所有插件（官方 / 第三方）都通过同一套 `window.mytools.*` API 消费宿主能力。
4. **能力优先于 API 名单**：规范以"能力（capability）"为最小单位，API 只是能力的调用方式。

## 2. 插件形态与目录布局

插件 = 一个目录，含清单与入口页面。

```
my-plugin/
├── plugin.json          # 清单（必填）
├── index.html           # 入口页面（必填，路径由 main 指定）
├── icon.png             # 可选：原生图标
├── preload.js           # 可选：入口前预加载脚本
└── assets/              # 可选：任意静态资源
```

- 入口页面运行在**沙箱 iframe**（`allow-scripts allow-same-origin allow-forms`，无 `allow-top-navigation` / `allow-popups`）。
- 插件页面由宿主**本地回环 HTTP** 提供（`http://127.0.0.1:<port>/<token>/…`）；不使用 `file://`（WebView2 不允许在 http 源内嵌 `file://` 帧）。
- 插件不得假设独占某个 origin；同插件多个页面共享同一 token 前缀。
- 消息桥仅接受绑定 iframe 的 `contentWindow` 与预期 origin；回复及生命周期消息使用明确的目标 origin。插件端仅接受父窗口及其 origin 发来的消息。
- 每次挂载固定插件身份；能力调用携带宿主创建的运行会话，后端校验会话、调用窗口和已启用插件。新会话替换同窗口旧会话，插件消息不得自行指定身份。

## 3. 清单规范 v1

### 3.1 完整示例

```json
{
  "name": "mytools-plugin-demo",
  "pluginName": "演示插件",
  "description": "演示插件",
  "version": "0.1.0",
  "author": "you",
  "main": "index.html",
  "icon": "icon.png",
  "apiVersion": 1,
  "origin": "custom",
  "capabilities": ["clipboard.write", "shell.open"],
  "permissions": { "exec": false },
  "features": [
    { "code": "run", "explain": "运行插件", "cmds": ["demo", "示例"] }
  ]
}
```

### 3.2 规范字段

| 字段 | 类型 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `name` | string | ✅ | — | 唯一标识（包名风格） |
| `version` | string | ✅ | — | 语义化版本 |
| `description` | string | ❌ | `""` | 一句话描述 |
| `author` | string | ❌ | — | 作者 |
| `main` | string | ✅ | — | 入口文件（相对插件目录） |
| `icon` | string | ❌ | — | 原生图标相对路径（PNG/SVG 等，≤512KB），加载后转 base64 data URI |
| `apiVersion` | number | ❌ | `1` | 需要的宿主能力主版本；高于宿主版本 → 激活时报不兼容 |
| `origin` | string | ❌ | 宿主判定 | `official` / `custom`；**仅作提示，授权以宿主判定为准** |
| `capabilities` | string[] | ❌ | `[]` | 声明需要的能力 id（§4） |
| `permissions` | object | ❌ | `{}` | 显式提权声明，如 `{ "exec": true }`；`true` 仍需用户确认 |
| `features` | array | ✅ | — | 可被搜索到的功能列表（§3.3） |
| `pluginName` | string | ❌ | — | 展示名（生态清单常用字段） |
| `hostSearch` | string | ❌ | — | 声明复用宿主顶部搜索，值为占位文字；不声明时使用插件自己的搜索框 |
| `preload` | string | ❌ | — | 入口前预加载脚本（相对路径） |
| `homePage` | string | ❌ | — | 插件主页（展示用） |

**未知字段**：宿主不解析、不报错，原样保留；插件私有字段（描述、图标路径等）走这条通道。

### 3.3 `features[]`

| 字段 | 类型 | 说明 |
|---|---|---|
| `code` | string | 插件内唯一功能码，路由与调用 dispatch key |
| `explain` | string | 功能说明（搜索结果副标题） |
| `cmds` | string[] | 搜索别名，可中英文混写；非字符串项忽略 |
| `icon` | string | 功能级图标（可选） |

## 4. 能力模型

### 4.1 能力 id 命名

`<域>.<动作>`，全小写，域用点分命名空间（**大写字母会被注册表拒绝，能力等于不存在**）：

| 域 | 语义 | 权限级别 |
|---|---|---|
| `clipboard.*` | 剪贴板读写 | read / write / exec |
| `shell.*` | 打开路径 / URL / 定位文件 | write |
| `storage.*` | 键值存储（插件私有） | read / write |
| `system.*` | 路径、网卡、通知、系统信息 | read / write |
| `window.*` | 主窗显隐、尺寸、位置 | write |
| `app.*` | 启动应用、查应用索引 | read / write |
| `net.*` | 网络请求 | exec |
| `fs.*` | 文件读写、目录授权 | exec / write |
| `plugin.*` | 插件间通信、打开其它插件 | write |

### 4.2 首版能力目录

| 能力 id | 参数 | 返回 | 权限 | 备注 |
|---|---|---|---|---|
| `clipboard.write` | `{ text }` / `{ dataUrl }` | — | write | 文本与图片 |
| `clipboard.read` | — | `{ text }` | exec | 需授权 |
| `clipboard.history` | `{ limit? }` | `ClipboardEntry[]` | exec | 需授权；读剪贴板历史 |
| `clipboard.remove` | `{ id }` | `true` | write | 需授权；删除一条历史 |
| `shell.open` | `{ path \| url }` | — | write | 路径 / URL 两种形态 |
| `shell.reveal` | `{ path }` | — | write | 资源管理器定位 |
| `storage.get` | `{ key }` | `unknown` | read | 插件私有空间 |
| `storage.set` | `{ key, value }` | — | write | 插件私有空间 |
| `storage.del` | `{ key }` | `boolean` | write | |
| `system.path` | `{ name }` | `string` | read | home/appData/config/temp/desktop |
| `system.netinterfaces` | — | `NetworkInterface[]` | read | 网卡 / IP / 网关 / DNS / MAC |
| `system.notify` | `{ title, body }` | — | write | 系统通知 |
| `window.show` | — | — | write | 唤出主窗 |
| `window.hide` | — | — | write | 隐藏主窗 |
| `window.resize` | `{ width, height }` | — | write | 分离窗口自适应 |
| `app.launch` | `{ target }` | — | write | 启动应用 |
| `app.list` | — | `ListedApp[]` | read | 应用索引 |
| `net.fetch` | `{ url, method?, headers?, body? }` | `{ status, body, headers }` | exec | 需授权；仅 http/https |
| `fs.read` | `{ path }` | `{ content }` | exec | 需授权；插件沙箱或已授权目录内 |
| `fs.write` | `{ path, content }` | — | write | 需授权；插件沙箱或已授权目录内 |
| `fs.pickdirectory` | — | `{ path }` | write | 弹系统目录对话框并登记目录授权 |
| `fs.grants` | — | `{ grants[] }` | read | 列出自己的目录授权 |
| `fs.revokegrant` | `{ path? }` | `{ removed }` | write | 撤销自己的目录授权 |
| `fs.search` | `{ path, name?, content?, maxResults?, maxDepth?, maxFileBytes? }` | `{ entries, truncated, scanned, skipped }` | read | 仅授权目录；结果、深度和扫描量有上限，不跟随符号链接 |
| `fs.renamemany` | `{ entries: [{ from, to }] }` | `{ renamed, failed }` | dangerous | 同目录普通文件；整批预校验；不覆盖目标；遇错停止 |
| `fs.searchstatus` | — | `{ running, instances }` | read | 检测本机 Everything 实例 |
| `fs.globalsearch` | `{ query, offset?, limit?, sort?, instance? }` | `{ entries, total, offset }` | exec | 查询 Everything 索引；不读取文件内容 |
| `fs.searchpreview` | `{ path }` | `{ kind, text?, bytes?, mime?, truncated? }` | exec | 仅当前搜索结果文件；文本前 20KB、图片不超过 4MB |
| `fs.searchicons` | `{ paths }` | `[{ path, icon }]` | read | 每批最多 16 个当前搜索结果，返回文件、目录或快捷方式目标的 Windows 图标；失败为 null |
| `fs.pickfiles` | `{ directories? }` | `{ entries }` | write | 系统选择文件 / 文件夹；明确授权所在目录 |
| `fs.importdropped` | — | `{ entries }` | write | 当前窗口最近一次系统拖放，60 秒有效、仅一次；授予所在目录，不接受插件提交路径 |
| `system.networkdetails` | — | 完整报告文本 | read | 本机 Windows ipconfig /all；系统 OEM 编码解码 |
| `fs.exporttext` | `{ name?, content }` | `{ path }` 或 `{ cancelled: true }` | write | 仅写用户在系统保存对话框选择的位置；最多 8MB |
| `screen.pickcolor` | — | `{ hex, rgb, x, y }` | exec | 读取当前光标所在像素；Windows |

`fs.*` 的可访问范围 = 插件沙箱（`<data>/plugin-files/<plugin-id>`，`<data>` = `%LOCALAPPDATA%\MyTools\data`）
∪ 用户已授权目录（见 §5.5）。插件 id 会做文件系统安全化；相对路径中的盘符、`..` 与空路径一律拒绝；绝对路径必须落在用户已授权目录内。`system.path` 里 `appData` 解析为 `<data>`、`config` 解析为配置根（`%APPDATA%\MyTools`）。

### 4.3 能力调用的错误模型

```ts
interface MyToolsError {
  code:
    | "CAPABILITY_DENIED"    // 权限不足（含具体能力 id）
    | "CAPABILITY_UNKNOWN"   // 能力不存在
    | "API_VERSION"          // 宿主能力版本过低
    | "INVALID_ARGS"         // 参数不合法
    | "HANDLER_FAILED"       // 能力执行失败
    | "UNAVAILABLE";         // 宿主不可用（非桌面环境）
  message: string;
  capability?: string;
  consentRequired?: boolean; // 需要用户确认后才可继续
}
```

规范面**不抛裸异常**：所有失败以 `MyToolsError` 形式 reject，插件可按 `code` 分支处理。

### 4.4 授权确认流程

声明 `hostSearch` 的插件通过 `onPluginReady` 的 `hostSearch` 判断当前窗口是否提供顶部搜索，通过 `window.mytools.onSearch(({query,key}) => ...)` 接收输入与 Enter / 方向键。输入不进入全局应用搜索。分离窗口没有公共顶部栏，插件需保留自己的搜索框。

插件通过 `window.mytools.onFilesDropped(callback)` 接收当前窗口原生文件拖入通知，再调用已声明的 `fs.importdropped` 消费宿主系统记录；通知不包含可伪造的授权路径。后续文件操作仍受目录授权与能力确认约束。

当策略判定为"需要确认"（`Prompt`）且没有对应的授权记录时：

1. 宿主返回 `CAPABILITY_DENIED` + `consentRequired: true`；
2. 前端弹出确认框，说明**哪个插件**要使用**哪个能力**；
3. 用户允许 → 写入授权记录（按插件 + 能力粒度）→ 原调用重试一次；
4. 用户拒绝 → 调用以 `CAPABILITY_DENIED` 结束。

授权记录可在设置中查看并逐条撤销；撤销后下次调用重新询问。

## 5. 权限与授权模型

### 5.1 权限级别

| 级别 | 含义 | 例子 |
|---|---|---|
| `read` | 读取宿主状态/私有数据 | `system.path`、`storage.get` |
| `write` | 修改宿主状态/外部世界 | `clipboard.write`、`shell.open`、`fs.write` |
| `exec` | 触发外部进程/网络 | `net.fetch`、`fs.read`、`clipboard.history` |
| `dangerous` | 不可逆破坏性操作 | 批量删除、进程终止类 |

### 5.2 判定顺序

```
能力请求
 → 能力是否已注册？否 → Deny(能力不存在)
 → apiVersion 是否满足？否 → Deny(版本不兼容)
 → 插件声明的 capabilities 是否覆盖？否 → Deny(未声明)
 → origin 默认策略 → Allow / Prompt / Deny
 → 用户已授权？→ Allow / Prompt
```

### 5.3 默认策略

| origin | read | write | exec | dangerous |
|---|---|---|---|---|
| `official` | ✅ | ✅ | ✅ | Prompt |
| `custom` | ✅ | Prompt | Prompt | Deny |

宿主可在能力注册时通过 `with_custom_consent()` 明确允许受目录边界保护的危险能力进入 `custom` 用户确认流程。目前仅 `fs.renamemany` 使用此入口；未授权时不执行，授权后仍逐项检查目录授权、符号链接与覆盖冲突。此标记不是插件清单字段，插件不能自行开启；其他危险能力继续默认拒绝。

**origin 由宿主按来源判定**，不由插件自报：装在用户插件目录里的插件一律 `custom`；`official` 只赋予产品自带的插件。清单里的 `origin` 字段只能把来源**收紧**为 `custom`，不能提权。

- 用户授权按**能力 id** 粒度持久化，可随时撤销。
- `Prompt` 以一次性确认弹窗呈现，说明能力名与目标插件。
- 拒绝必须返回**可读错误**（`CAPABILITY_DENIED` + 原因），不静默失败。

### 5.4 声明式能力

清单 `capabilities` 非空时，插件**只能**调用声明过的能力（未声明 → `CAPABILITY_DENIED`）。为空表示"不限制"（兼容旧插件）。

### 5.5 目录授权（真实文件）

自装插件默认只能访问自己的沙箱目录。要操作用户真实文件，流程是：

1. 插件调用 `fs.pickdirectory` → 宿主弹**系统目录对话框**（用户亲自选目录）；
2. 用户选定 → 写入 `directory-grants.json`（`{ plugin, path }`）；
3. 此后该插件的 `fs.read` / `fs.write` 可在该目录及其子目录内操作，越界一律拒绝；
4. 撤销入口：「偏好设置 → 目录授权」。

授权记录按插件隔离，插件之间不共享。目录比较大小写不敏感并要求分隔符边界（`C:\a\b` 不匹配 `C:\a\bc`）。

## 6. 生命周期

```
install ──► enable ──► activate ──► enter ──► (out) ──► deactivate ──► disable
   │                                        ▲
   └────────────── uninstall ◄──────────────┘
```

| 阶段 | 宿主行为 | 插件可感知 |
|---|---|---|
| `install` | 解包、校验清单、写入启用状态 | — |
| `enable` | 纳入扫描结果，进入搜索与插件中心 | — |
| `activate` | 解析清单、校验能力、挂载视图 | `window.mytools.onPluginReady` |
| `enter` | 派发环境（code / type / payload） | `onPluginEnter(env)` |
| `out` | 插件离开视图 | `onPluginOut()` |
| `deactivate` | 卸载视图、清理资源 | — |
| `disable` | 从扫描结果移除 | — |
| `uninstall` | 删除目录与状态 | — |

- 视图挂载采用 `PluginHost` 抽象。
- 插件页面崩溃由**错误总线**接管：`error` / `unhandledrejection` → 宿主收集 → 面板展示。

## 7. 宿主 API（`window.mytools.*`）

```ts
interface MyToolsAPI {
  readonly apiVersion: number;             // 宿主能力契约版本

  // 能力调用（统一入口）
  call<T = unknown>(capability: string, args?: Record<string, unknown>): Promise<T>;

  // 生命周期
  onPluginReady(cb: (env: MyToolsEnv) => void): void;
  onPluginEnter(cb: (env: MyToolsEnv) => void): void;
  onPluginOut(cb: () => void): void;

  // 环境
  getEnv(): MyToolsEnv;
}

interface MyToolsEnv {
  code: string;                                  // feature code
  type: "text" | "img" | "files" | "regex" | "over" | "window";
  payload?: unknown;
  plugin?: string;
  apiVersion: number;                            // 宿主能力版本
}
```

- 窗口控制、通知、网络等一律通过 `call()` 走能力 id（`window.show`、`system.notify`、`net.fetch` …）。
- 规范面**不暴露**底层 IPC 细节，插件不得直接访问宿主内部对象。
- 非桌面环境（浏览器预览）返回 `UNAVAILABLE`，不抛底层异常。
- 宿主窗口与插件 iframe 都提供 `window.mytools`：iframe 侧由注入脚本代理到宿主，宿主按插件 id 传 scope，因此授权判定用的是插件自己的来源。

### 7.1 版本协商

- 宿主在注入 API 时附带 `apiVersion`；插件可读取 `window.mytools.apiVersion` 判断可用能力。
- `call()` 请求未注册能力 → `CAPABILITY_UNKNOWN`；宿主版本低于插件声明 → 激活阶段即报 `API_VERSION`，插件不进入运行态。

## 8. 打包与安装

| 形态 | 来源 | 说明 |
|---|---|---|
| 目录 | 本地文件夹 | 拷贝到插件根目录 |
| 归档 | `.zip` / `.tgz` / `.tar.gz` | 自动识别扩展名；tgz 需含单一 `package/` 根 |
| 包管理器 | `name[@version]` | 解析 registry → 下载包 → 安装 |

**安全约束**

- 拒绝 `..`、绝对路径、超 20000 条目（zip-slip）。
- 目录名取"首个安全名"，不信任归档内路径。
- 卸载要求目标目录含合法清单，防止误删。
- 安装后校验清单与 `apiVersion`，不兼容时显式标注。

## 9. 版本策略

| 变更 | 是否提升 `apiVersion` | 兼容保证 |
|---|---|---|
| 新增能力 | 否（次版本） | 旧插件不受影响 |
| 新增可选清单字段 | 否 | 旧插件不受影响 |
| 能力语义变更 / 移除 | **是**（主版本） | 旧插件在激活时明确报 `API_VERSION` |
| 清单必填字段新增 | **是** | 同上 |

- 废弃流程：标记 `deprecated` → 至少保留一个次版本 → 下一个主版本移除。

## 10. 插件开发者检查清单

- [ ] 清单含 `name` / `version` / `main` / `features`
- [ ] `features[].code` 唯一；`cmds` 覆盖中英文常见叫法
- [ ] 只用 `window.mytools.*`，不触碰宿主内部对象
- [ ] 所有能力调用按 `MyToolsError.code` 分支处理失败
- [ ] 声明 `capabilities`，不申请用不到的能力
- [ ] 不用 `localStorage` 持久化（用 `storage.*`）
- [ ] 崩溃能被宿主捕获：不吞 `unhandledrejection`

## 11. 相关文档

- [02-architecture](./02-architecture.md) — 两层模型与扩展点
- [05-data-and-storage](./05-data-and-storage.md) — 插件私有存储与状态
- [06-roadmap](./06-roadmap.md) — 实施规划
- [07-testing](./07-testing.md) — 测试策略
- [08-design-decisions](./08-design-decisions.md) — 设计决策记录
- [10-plugins-and-tools](./10-plugins-and-tools.md) — 插件与工具包开发指南
