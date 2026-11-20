# 资源管理与电脑助手

状态：已实现。宿主能力在 main 分支，电脑助手 0.1.1 可通过 npm 安装。相关入口：资源管理页面、独立 mytools-computer-assistant 插件。

资源管理位于开发者之后、偏好设置之前，展示本应用进程组 CPU、内存工作集、活动插件实例，以及安装文件、插件私有文件、共享配置、日志、备份和 WebView 数据目录。可打开目录、确认关闭插件独立窗口，不删除文件或结束进程。

电脑助手使用公开 system.resources 能力展示整机 CPU、内存、页面文件、逐网卡速率、磁盘余量、进程排行与系统信息。GPU、温度和磁盘实时读写速度本版不提供。提醒默认关闭，仅页面打开且可见时工作；连续三个有效样本超出阈值才提示，系统通知间隔至少五分钟。

## 统计口径

- CPU 统一按整机 0—100% 计算；进程 CPU 除以逻辑核心数。首次及间隔超过十秒后的首个采样处于预热期。
- MyTools 总卡片仍为工作集合计，共享页可能重复；电脑助手排行默认私有工作集，允许切换为含共享部分的工作集。私有工作集直接读取 Windows PROCESS_MEMORY_COUNTERS_EX2.PrivateWorkingSetSize，不使用 PrivateUsage（提交量）替代。读取失败或旧系统不支持时返回 null；应用部分可读时展示已知下限及覆盖数量，不显示为零。多个插件共享 WebView，不按插件数量均摊内存。
- 应用依据同一可执行路径的父子进程链分组，校验父进程启动时间，防止已复用 PID 误关联；独立实例、不同路径同名程序、路径未知的进程不强行合并。仅相同服务程序的服务/客户端在 SCM 配置核实后允许跨进程树合并，Everything 可展开服务与客户端。分组不承诺完整复制 Windows 任务管理器的包身份、窗口或辅助进程归类方式。
- 本应用进程组由当前 PID、主 WebView 环境 BrowserProcessId 及其 WebView 后代组成；不把启动的其他应用或其他应用的 WebView 计入。取不到环境标识时沿父子关系查找 WebView 子树，并提示可能不完整。
- Umi-OCR、Everything、llama-server 等外部进程独立列出；未验证其服务端口可用性。
- 网络按具体网卡展示，用累计收发字节之差计算速率，不把 VPN、虚拟网卡和物理网卡直接相加。
- 磁盘统计文件逻辑长度，不代表 NTFS 实际分配空间。共享 plugin-store.json 按插件命名空间估算条目逻辑长度；物理文件仅在配置目录计一次。不同目录可能重叠，不提供误导性总和。

## 开销与边界

两个界面共享一秒采样缓存，按需工作，没有常驻采样线程；磁盘容量刷新间隔三十秒。资源管理每两秒更新进程信息，切走页面后停止。目录扫描缓存六十秒，支持手动重扫；总预算十秒、二十五万节点。跳过符号链接和重解析点；无法读取、跳过或触及上限时标为部分扫描。

## 公开能力

`await mytools.call('system.resources', {})`，清单声明 `system.resources`，需要能力授权。该只读能力返回进程名称与 PID，不返回命令行、文件内容或任意路径扫描接口。

结果 `version:1`、`sampledAt`（Unix 毫秒）、`warming`，以及 `machine`、`networks`、`disks`、`topMemory`、`topCpu`、`mytools`、`externalServices`。字节单位均为 B，速率为 B/s，CPU 为整机百分比。machine 包含 cpu/cores/cpuName/totalMemory/usedMemory/totalSwap/usedSwap/uptime/os；gpu/temperature 为 null。进程行包含 pid/parent/name/cpu/memory；网卡行包含 name/receivedPerSecond/transmittedPerSecond；磁盘行包含 name/mount/total/available。旧版 topMemory/topCpu 仍按原口径最多二十项。插件应处理授权拒绝、旧宿主不支持能力和无数据，不在关闭后持续调用。

### 应用汇总扩展

新增 `processDetailsVersion:2`、`applications`（完整应用组）、`externalApplications`（外部服务组），不改变原 version:1 字段含义。组包含 id/name/rootPid/processCount/cpu/memory/privateMemory/knownPrivateMemory/privateMemoryKnown/association/processes；每个进程恰好归属一个应用组。先汇总所有进程再由前端筛选、排序和取前二十，不能先截断进程再分组。

进程新增 privateMemory（可为 null）和 role。组 privateMemory 仅在全组可读时为精确合计，否则为 null；knownPrivateMemory 为已知下限，privateMemoryKnown 为可读取进程数量。association 为 same-executable-tree、verified-service-executable 或 single-process。组 id 随根 PID 和启动时间变化；不向插件返回可执行路径、完整命令行、服务启动参数或任意文件内容。私有工作集接口要求 Windows 10/11 较新累积更新，不支持时展示不可用，可手动选工作集。

测试覆盖进程树归属、循环关系、目录扫描上限、共享存储命名空间、页面停采与恢复，以及插件的安全渲染、暂停/隐藏/关闭、拒绝授权、提醒阈值与冷却、设置保存失败。
