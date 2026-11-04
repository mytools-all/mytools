# 案例：MyTools Demo

状态：与 mytools-demo 0.1.1 源码对应；npm 是否可安装以索引 published 为准，当前请用本地源码或 tgz，不因版本号存在就假定已发布。

独立源码：[mytools-all/mytools-demo](https://github.com/mytools-all/mytools-demo)。在完整工作区中位于 plugins/mytools-demo。此案例无需 Vue、构建工具、Node 运行时或宿主源码。

## 文件与调用路径

plugin.json 声明运行身份 mytools-demo、入口 index.html、图标 icon.svg、搜索别名和五项能力；package.json 的 npm 名称 @mytools-all/mytools-demo 与之不同但版本相同。index.html 提供五个按钮，共用 run(id,args)，通过 window.mytools.call 调用并将结果写入 pre.textContent。

| 按钮 | 能力 | 参数 | 结果 |
|---|---|---|---|
| 复制文本 | clipboard.write | {text:'hello from mytools-demo'} | 完成提示 |
| 读取剪贴板 | clipboard.read | {} | 显示宿主返回值 |
| 打开链接 | shell.open | {url:'https://github.com/mytools-all/mytools'} | 完成提示 |
| 读取主目录 | system.path | {name:'home'} | 路径字符串 |
| 列出应用 | app.list | {} | 应用数组 |

无宿主桥接或授权被拒绝时显示调用失败。没有自行模拟 IPC、指定插件身份或绕过权限。此 demo 只有点击动作，无初始化存储需求；有持久化的插件应按指南等待 ready 后读取并校验数据，而不是复制此处的无状态实现。

## 运行步骤

在 demo 目录执行 npm test，再 npm pack --dry-run / npm pack。宿主开发者页面安装 tgz；目录方式将完整插件目录放入“打开插件目录”指向的根目录后“重新扫描”，从主页搜索“示例插件”进入。按需接受授权；读取剪贴板仅使用自己的测试文本，应用列表和路径不要提交到仓库。

demo.test.mjs 使用 node:vm 执行实际 HTML 脚本，模拟五个按钮与桥接，验证每个动作使用清单声明的能力；package.test.mjs 验证清单版本、npm 名称和入口资源。测试不替代宿主内授权和独立窗口验收。

## 从 Demo 到业务插件

保留清单、离线资源和 call/catch 模式；更换身份、图标、入口与能力。按业务拆分 app.mjs（DOM）、model.mjs（纯逻辑）、storage.mjs（保存事务），不要照搬不需要的剪贴板或系统权限。“我的日历”仅使用 storage.get/set，以验证普通业务无需修改宿主即可实现。
