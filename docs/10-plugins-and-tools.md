# 插件与工具包

宿主和插件是独立仓库。所有工具均为可单独安装的插件包，不编译进宿主，也不在启动时播种。集中仓库链接与 npm 索引见 [mytools-home](https://github.com/mytools-all/mytools-home)。

入门统一使用 [插件开发指南](plugin-development/README.md) 和 [Demo 案例](plugin-development/demo-walkthrough.md)，本文保留分发与索引职责说明。

## 包结构

每个插件包含 plugin.json、package.json、index.html、页面脚本、样式和许可证。依赖资源放在同一包中，使用相对路径；不依赖宿主源码或其他插件目录。

plugin.json.name 是稳定运行时身份，例如 mytools-search；package.json.name 是 npm 名称，例如 @mytools-all/mytools-search。两者版本一致，本次起点为 0.1.0。宿主优先读取 plugin.json，不用 npm 作用域替换插件身份。

## 安装

插件商店目录项使用固定 npm 包版本安装；开发者页面支持 npm spec、本地目录、zip 和 tgz。默认安装目录位于应用数据下的 plugins，可通过设置或 MYTOOLS_PLUGINS_DIR 覆盖。克隆插件仓库后，仍需在宿主中安装。

## 开发与权限

具体清单、能力、会话、目录授权和生命周期见 [插件规范](04-plugin-spec.md)。插件通过宿主桥接调用能力，不直接调用 Tauri IPC。npm 安装与本地安装都按普通插件处理，不能自行声明可信身份。写入、网络、危险操作及目录访问遵循宿主授权策略。

每个插件自行维护其离线资源及第三方许可证，在自身仓库运行 npm test。发布前用 npm pack --dry-run 检查文件，再用 npm publish --access public 发布；认证与二次验证在本机完成。

## 索引

mytools-home 的 plugins/database.json 是商店分类和基本信息的唯一维护入口。schemaVersion 为 2，每项包含 name、npm、version、published、title、description、category、author、license、repository、features、keywords、notes、capabilities；可选 icon、homepage、reference、replaces。

插件商店进入时异步抓取索引，支持手动刷新。身份与 npm 名称必须合法、版本必须明确，重复身份或不合法链接会拒绝整次更新；失败保留上次有效缓存或离线快照。分类直接从索引派生。离线快照通过 pnpm catalog:sync 自动同步，不单独维护分类、介绍或使用说明。

商店直接安装 npm 包到统一插件根目录，不弹目录选择框。本地目录和压缩包导入仅保留在开发者页面。已安装详情提供打开、启停、打开目录及确认卸载；卸载插件程序不删除独立插件存储。

索引 capabilities 仅供展示，不授予权限。已安装插件的实际能力和版本以本地清单为准，调用继续通过宿主会话和权限校验。demo-hello 已更名为 mytools-demo；旧 npm 包提供迁移提示，旧安装可以卸载后从新条目安装。
