# 02 · 系统架构

> **状态**：设计稿（持续修订）· 相关：[01-requirements](./01-requirements.md) · [04-plugin-spec](./04-plugin-spec.md)
> **本文回答**：系统分哪几层？每层负责什么、边界在哪？扩展点在哪里？插件从安装到调用能力经历什么流程？

## 1. 架构总览

采用**两层架构**：底层只提供机制，应用层只消费机制。插件体系不再有生态兼容层——
外部生态的方言、生命周期与运行时依赖无法在 WebView 宿主中可靠承载，因此工具一律以
**自研插件包**形式交付（见 [10-plugins-and-tools](./10-plugins-and-tools.md)）。

```
┌───────────────────────────────────────────────────────────────┐
│ L2 应用层  Applications                                        │
│  内置命令 │ 官方插件 │ 第三方插件（自研工具包）                 │
│  只消费规范 API；不直接触碰 L1 内部实现                        │
└───────────────▲───────────────────────────────────────────────┘
                │ 依赖（只向下）
┌───────────────┴───────────────────────────────────────────────┐
│ L1 基础层  Foundation                                           │
│  能力注册表 │ 清单规范模型 │ 运行时载体 │ 权限策略              │
│  资源服务 │ 错误总线 │ 生命周期 │ apiVersion 契约 │ 目录授权    │
│  机制唯一；不认识任何具体插件或生态                            │
└───────────────────────────────────────────────────────────────┘
```

| 层        | 一句话                               | 判定标准                   |
| --------- | ------------------------------------ | -------------------------- |
| L1 基础层 | 提供**机制**，不认识任何具体插件 | 出现具体插件/生态专名即越界 |
| L2 应用层 | 提供**用户可用的插件与工具**     | 不含通用机制，只含具体功能 |

## 2. 分层职责与边界

### 2.1 L1 基础层

**负责**

- **能力注册表**：`capability id → { 参数 schema, 权限级别, handler }`。宿主只做注册、鉴权、路由、审计，不写业务分支。
- **清单规范模型**：统一 `Manifest` 结构（name / version / main / features / apiVersion / capabilities / permissions）。
- **运行时载体**：iframe 宿主 + 独立窗口（插件页可分离成单独窗口）。
- **资源服务**：本地回环静态服务（token 前缀 + 目录穿越防护），为插件页面与资源提供 http 源。
- **权限策略**：`Policy` 抽象（能力 → 放行 / 需用户授权 / 拒绝）。
- **目录授权**：用户为某个插件挑选真实目录，授权记录持久化，文件能力只在授权目录内生效。
- **错误总线**：插件侧 `error` / `unhandledrejection` 统一采集、去噪、上报、展示。
- **契约版本**：`apiVersion` 协商——宿主按版本决定暴露哪些能力。

**不负责**：不解析插件私有字段、不做单个插件的特殊分支、不决定插件 UI 形态细节。

### 2.2 L2 应用层

**负责**

- **内置命令**：编译期硬编码的快速入口（计算器、编解码、剪贴板等），无需安装即用。
- **官方插件**：由维护者独立发布的插件，本版本通过普通安装通道分发。
- **第三方插件**：用户安装的自研工具包（`独立插件仓库` 目录形态），装完即用，能力受权限策略约束。

每个插件声明：`origin`（official / custom）、所需能力、功能入口（features）。

**不负责**：不含通用机制；不绕过能力注册表直连 L1 内部。

## 3. 层间依赖规则

1. **只能向下依赖**：L2 → L1；禁止反向依赖。
2. **跨层调用只走能力注册表**：任何插件请求宿主能力，都以 `capability id` 发起。
3. **L1 不感知插件**：L1 代码中不允许出现具体插件专名。

### 3.1 宿主内部职责

两层模型定义产品边界；宿主内部按以下职责组织，不要求每个模块独立为 crate。

| 模块 | 职责 | 依赖约束 |
|---|---|---|
| `bootstrap` | 初始化顺序、状态装配、后台任务与桌面集成 | 可装配所有宿主模块；业务模块不反向调用启动入口 |
| `commands` | IPC 入参、结果映射与宿主事件适配 | 请求交给运行时或业务服务；不持有独立的一套插件状态 |
| `runtime` | 会话身份、能力声明校验、能力执行调度 | 从插件管理服务获取当前状态；执行前校验启用状态与会话 |
| `services/capabilities` | 按领域注册能力及其 handler | 注册时声明 ID、权限与处理器，统一进入核心注册表 |
| `services/plugins` | 插件发现、来源、启停、安装与目录切换 | 不处理 IPC 事件；变更通知由桌面适配入口发出 |
| `services/apps` | 应用索引快照、扫描、启动目标解析与应用启动 | 宿主 IPC 与插件能力共用同一状态与执行实现 |
| `services/search` | 合并内置入口、应用索引与插件结果 | 一次请求使用同一份插件快照；阻塞工作交给后台执行 |
| `services/settings` / `clipboard` / `system` | 设置持久化、剪贴板访问、系统目录解析 | 不依赖 IPC 模块；前端命令与能力 handler 复用服务 |
| `infrastructure` | 路径、日志与 Windows 系统接口 | 为服务提供平台实现，不负责插件权限策略 |
| `mytools-core` | 清单、注册表、权限、目录授权、运行会话与纯业务规则 | 零 Tauri 依赖 |

业务服务、运行时与平台设施均不得依赖 `commands`，也不得声明 Tauri IPC 命令；通过架构边界测试约束。桌面命令负责 Tauri 窗口、托盘、快捷键的交互适配。

运行会话在视图销毁或窗口关闭时回收；同窗口创建新会话会使旧会话失效。
同步 handler 统一交给有并发上限的阻塞线程池，线程内在执行前校验身份与当前插件状态。
执行容量耗尽返回明确错误，不建立无界等待队列；已开始的系统操作不承诺可取消。
注册表本身是能力 ID、权限和处理器的运行时真源，注册失败应向启动入口返回错误，避免以残缺能力集启动。

前端职责：`App.vue` 组合布局，`useAppShell` 管理初始化和桌面事件订阅；
`usePluginRuntime` 管理插件挂载、切换与销毁；桥接模块只处理消息协议；错误总线和授权队列分别管理各自状态。

插件清单按窗口缓存，由壳订阅插件变更事件并刷新；页面入口复用快照，并发首次加载共用请求。
搜索结果不等待系统图标解析；图标按可见结果异步加载、缓存并限制并发。
插件 iframe 在加载阶段保持完整布局，资源就绪后派发进入事件；退出时仍销毁桥接与会话。
授权对话框属于窗口壳公共区域，主窗口与分离插件窗口都可呈现。并发授权按请求顺序处理，插件销毁会取消该实例的待授权请求。

## 4. L1 基础层设计

### 4.1 扩展点一：能力注册表

```rust
pub struct Capability {
    pub id: CapabilityId,          // 例如 "clipboard.write"
    pub permission: Permission,    // read | write | exec | dangerous
    pub params: ParamSchema,       // 参数与返回形状
    pub handler: Handler,           // 实际执行
}

pub trait CapabilityRegistry {
    fn register(&mut self, cap: Capability);
    fn get(&self, id: &str) -> Option<&Capability>;
    fn call(&self, id: &str, args: Args, ctx: CallContext) -> Result<Value, CapabilityError>;
}
```

- 新增一个能力 = 注册一条，**不改核心**。
- 调用一律带 `CallContext`（插件 id、来源、窗口 label），供权限策略与审计使用。
- 能力 id 命名规范见 [04-plugin-spec](./04-plugin-spec.md) §4。

### 4.2 扩展点二：清单模型

规范 `Manifest` 只含跨插件稳定字段；插件私有字段（`pluginName` / `logo` / `cmds` …）在解析时宽容吸收，
不进规范模型。

### 4.3 扩展点三：运行时载体

```rust
pub trait PluginHost {
    fn kind(&self) -> HostKind;      // iframe | window
    fn mount(&self, plugin: &ResolvedPlugin) -> Result<MountedView>;
    fn unmount(&self, view: MountedView);
}
```

### 4.4 扩展点四：权限策略

```rust
pub trait Policy {
    fn decide(&self, cap: &Capability, ctx: &CallContext) -> Decision;
}
enum Decision { Allow, Prompt, Deny }
```

默认策略见 [04-plugin-spec](./04-plugin-spec.md) §5.3。

### 4.5 扩展点五：分发源

```rust
pub trait Source {
    fn list(&self) -> Result<Vec<CatalogEntry>>;   // 目录 / 包管理器
    fn fetch(&self, spec: &str) -> Result<Package>; // 下载
}
```

### 4.6 扩展点六：目录授权

```rust
pub trait DirectoryGrant {
    fn grant(&self, plugin: &str, dir: &Path) -> Result<()>;   // 用户在系统对话框中选定
    fn is_granted(&self, plugin: &str, target: &Path) -> bool; // 路径在前缀边界内
    fn revoke(&self, plugin: &str, dir: Option<&str>) -> Result<usize, String>;
}
```

授权是**用户显式选择**的结果：插件调用 `fs.pickdirectory` 弹系统目录对话框，选中的目录
写入授权记录；此后该插件可在该目录内读写。撤销入口在「偏好设置 → 目录授权」。

### 4.7 契约版本

- `apiVersion`：宿主能力集合的主版本号。能力有增删时提升主版本。
- 插件声明的 `apiVersion` 高于宿主时，宿主在激活阶段**显式报不兼容**，不静默降级。
- 同一主版本内能力只增不改语义；废弃能力走 `deprecated` 标记 + 至少一个次版本保留期。

## 5. 插件体系设计

### 5.1 两种插件形态

- **内置命令**：编译期硬编码的快速入口（计算器、编解码等），无需安装即用。
- **插件**：目录 + 清单的通用形态，可发布新功能而不重编宿主。

### 5.2 来源标记

| origin     | 来源                 | 权限默认策略                             | UI 呈现        |
| ---------- | -------------------- | ---------------------------------------- | -------------- |
| `official` | 宿主信任来源（保留机制） | 放行（`dangerous` 除外）             | 官方标识       |
| `custom`   | 用户自行安装         | 仅 `read`；`write`/`exec` 需用户授权 | 显示"第三方"标识 |

`origin` 由**宿主按安装来源判定**，清单中的 `origin` 字段仅作提示，不作为授权依据。

### 5.3 能力声明与授权

- 插件在清单里声明 `capabilities`；声明后**只能**调用列出的能力。
- 未声明能力的调用一律拒绝并给出原因（见 [04-plugin-spec](./04-plugin-spec.md) §5）。
- 写类与网络类能力对自装插件需用户确认（一次性授权，记录在 `permissions.json`）。

### 5.4 不做什么

- 不为单个插件做特例分支。
- 不实现宿主不提供的平台能力（如主进程级系统能力）。
- 不做「生态兼容分级」：插件是否能运行由其声明与实际能力调用结果决定，不预先打分。

## 6. 关键数据流

### 6.1 安装 → 激活

```
选择插件（本地目录 / 压缩包 / npm 包）
  → 解包到插件根目录（路径安全校验）
  → 解析规范 Manifest
  → 校验：main 存在、apiVersion ≤ 宿主版本
  → 写入启用状态 → 扫描刷新 → 广播变更事件 → 前端刷新
```

### 6.2 激活（打开插件页）

```
命中功能 → 路由到插件视图
  → iframe 宿主挂载
  → 资源服务提供 http://127.0.0.1:<port>/<token>/<plugin>/…
  → 注入桥接脚本（window.mytools.*）
  → 宿主派发生命周期 enter（携带规范 env）
```

### 6.3 一次能力调用

```
插件：window.mytools.call("clipboard.write", { text })
  → postMessage 桥接 → 宿主
  → 查能力注册表（能力是否存在、是否在插件声明内）
  → 权限策略判定（origin + 权限级别 + 用户授权）
  → 执行 handler → 结果回传
```

## 7. 目标目录结构

```
src-tauri/
├── src/
│   ├── lib.rs                       # Tauri 插件和 IPC 注册
│   ├── bootstrap.rs                 # 启动装配
│   ├── commands/                    # IPC 适配入口
│   ├── runtime/                     # 身份校验与执行调度
│   ├── services/
│   │   ├── capabilities/            # 剪贴板、文件、网络等领域能力
│   │   ├── plugins/                 # 插件管理服务
│   │   ├── apps/                    # 应用索引与启动
│   │   ├── clipboard.rs             # 共用剪贴板访问
│   │   ├── search.rs                # 搜索合并与后台执行
│   │   ├── settings.rs              # 设置模型与持久化
│   │   └── system.rs                # 系统目录解析
│   └── infrastructure/              # 平台路径、日志、网络适配器
└── crates/mytools-core/src/          # 零 Tauri 依赖的规则与模型
src/
├── App.vue                          # 窗口布局
├── composables/useAppShell.ts        # 壳初始化与事件订阅
├── composables/pluginBridge.ts       # iframe 消息协议
├── runtime/                         # 插件生命周期、授权队列、错误总线
├── components/CapabilityConsentDialog.vue
└── views/tools/                     # 工具与插件呈现

```

## 8. 扩展成本对照

| 扩展目标           | 需要改动的层                                     | 改动量              |
| ------------------ | ------------------------------------------------ | ------------------- |
| 新增一个宿主能力   | 对应领域注册 ID、权限与 handler，补充契约与验证     | 修改领域模块与规范  |
| 新增一种运行时载体 | L1 实现 `PluginHost`                             | 1 个新文件          |
| 新增一种分发源     | L1 实现 `Source`                                 | 1 个新文件          |
| 新增一个官方插件   | 独立插件仓库 + npm 包 + 目录索引                         | 1 个插件目录        |
| 新增一个工具包     | 无需改宿主（写一个插件目录即可）                  | 1 个插件目录        |

## 9. 相关文档

- [01-requirements](./01-requirements.md) — 目标与判据
- [04-plugin-spec](./04-plugin-spec.md) — 插件规范（清单 / 能力 / 权限 / 生命周期 / API）
- [05-data-and-storage](./05-data-and-storage.md) — 数据与存储
- [06-roadmap](./06-roadmap.md) — 实施规划与进度
- [07-testing](./07-testing.md) — 测试策略
- [08-design-decisions](./08-design-decisions.md) — 设计决策记录
- [10-plugins-and-tools](./10-plugins-and-tools.md) — 插件与工具包开发指南
