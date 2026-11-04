# MyTools 文档

> 文档语言：**中文为主，技术术语、API 名、字段名、代码片段保留英文**。
> 本目录同时保存实现说明与历史设计。以 [项目上下文](PROJECT-CONTEXT.md)、[插件开发指南](plugin-development/README.md) 的已实现契约为开发入口；历史设计中的目标不自动表示已经实现。
> 每份文档顶部有**状态头**（状态 / 相关文档 / 本文回答什么）。

## 开始阅读

- 更换 AI 工具或接手项目：[项目上下文](PROJECT-CONTEXT.md)，并阅读仓库 AGENTS.md。
- 开发插件：[完整开发指南](plugin-development/README.md) → [Demo 案例](plugin-development/demo-walkthrough.md)。
- 核查文档是否可用：[日历插件验证](plugin-development/validation-calendar.md)。
- 当前交付边界：[交接说明](11-handoff.md)。

## 文档索引

| # | 文档 | 一句话内容 | 主要读者 |
|---|---|---|---|
| 01 | [需求分析](./01-requirements.md) | 为谁做、目标与判据、范围外、风险 | 全员 |
| 02 | [系统架构](./02-architecture.md) | **两层架构**、扩展点、插件体系、数据流 | 后端 / 架构 |
| 03 | [技术栈选型](./03-tech-stack.md) | 依赖清单、选型理由、平台约束 | 全员 |
| 04 | [插件规范](./04-plugin-spec.md) | 清单 v1、能力目录、权限、目录授权、生命周期、宿主 API | 插件作者 / 后端 |
| 05 | [数据与存储](./05-data-and-storage.md) | 存储布局、SQLite、配置与插件状态 | 后端 |
| 06 | [实施规划与进度](./06-roadmap.md) | 里程碑、验收标准、状态、进度同步约定 | 全员 |
| 07 | [测试策略](./07-testing.md) | 分层职责、关键清单、CI、门禁 | 全员 |
| 08 | [设计决策记录](./08-design-decisions.md) | 关键设计取舍（为什么这么定） | 全员 |
| 10 | [插件与工具包](./10-plugins-and-tools.md) | 官方插件、工具包形态、能力目录、授权模型 | 工具包作者 |
| 12 | [工具交互规范](./12-tool-interactions.md) | Rubick 参考交互、核心功能与平台边界 | 工具包作者 |
| 13 | [面板启动上下文](./13-launch-context.md) | 文件、截图与文本的统一采集和受控交付 | 插件作者 / 后端 |

## 两条阅读路径

**了解项目（30 分钟）**
1. [01-requirements](./01-requirements.md) — 目标与判据
2. [02-architecture](./02-architecture.md) §1–§3 — 两层模型与依赖规则
3. [06-roadmap](./06-roadmap.md) §1 — 里程碑与当前进度
4. [08-design-decisions](./08-design-decisions.md) — 为什么这么设计

**写工具包（1 小时，优先使用新指南）**

先读 [插件开发指南](plugin-development/README.md) 和 [Demo 案例](plugin-development/demo-walkthrough.md)，需要完整能力表时再查以下参考。

1. [10-plugins-and-tools](./10-plugins-and-tools.md) — 工具包形态、清单、宿主 API、能力目录、授权
2. [04-plugin-spec](./04-plugin-spec.md) — 清单、能力、权限、目录授权、生命周期
3. [04-plugin-spec](./04-plugin-spec.md) §10 — 开发者检查清单

## 维护约定

1. **单一事实源**：每个主题只在一份文档定义，其它文档引用而非复述。
2. **状态明确**：在开头写明是实现指南、设计目标还是验证记录，以及适用版本和相关入口。
3. **区分状态**：实现指南必须核对实际契约；设计目标明确标注未实现，历史流水交给 git。
4. **决策入档**：影响面较大的设计决定追加到 [08-design-decisions](./08-design-decisions.md)，不修改历史条目。
5. **里程碑驱动**：需求变更先改 [06-roadmap](./06-roadmap.md) 的里程碑与验收标准，再改对应专题文档。
6. **文档与代码同 PR**：规范/架构类改动必须与实现同 PR，或紧随其后（禁止滞后超过一个里程碑）。
