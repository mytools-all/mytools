# 便签入门模板

版本 0.1.0，API v1，教学源码，未发布。无需构建、无第三方依赖；运行时只使用宿主注入的桥接。开发测试与打包需要 Node.js 24+ 和 npm。

## 复制并运行

不要直接在宿主的文档目录开发你的正式插件。从 MyTools 工作区根目录执行：

```powershell
Copy-Item -Recurse ./mytools/docs/plugin-development/templates/mytools-notes ./plugins/mytools-notes-starter
cd ./plugins/mytools-notes-starter
npm test
npm pack --dry-run
npm pack
```

目标目录必须尚不存在，避免复制时产生嵌套目录。模板不是独立 Git 仓库；需要版本管理时在复制目录执行 `git init`。`private:true` 防止误发布，但不阻止本地 npm pack。正式开发时修改 plugin.json 的 name、pluginName、features 搜索词及 package.json 的 name，两个文件版本一致；勿复用已有插件身份。只有真正准备发布时才另行配置自己的 npm 作用域。

在 MyTools → 插件中心 → 开发者 → 安装插件包（.zip / .tgz），选择 `mytools-notes-starter-0.1.0.tgz`。从主页搜索“便签入门”，按宿主提示授权。源码改动后重新打包安装，再关闭并重开插件。仅用浏览器打开 HTML 会显示预览提示，不能保存数据。

## 文件怎么读

1. plugin.json：宿主识别、入口、权限、搜索别名。
2. index.html / style.css：界面、禁用状态、浅深主题和滚动。
3. model.mjs：load → edit → save 状态机，不依赖 DOM，可独立测试。
4. app.mjs：把按钮、文本框和宿主桥接接到状态机。
5. tests/：使用真实状态机和模拟存储，验证成功与失败路径。

## 权限、数据与边界

仅声明 storage.get、storage.set，不使用网络、文件、剪贴板、localStorage 或 IndexedDB。键为 notes.v1，值为 `{version:1,text:"内容"}`，不存在时为 null。数据交由宿主插件作用域存储；卸载插件后存储保留，重新安装相同身份可能读取原数据。

首次加载前不能编辑；读取失败或未知版本时禁止写入并允许重试。保存期间禁用编辑与重复保存；成功后才更新已保存状态，失败保留文本，可再次点击保存。重复 ready 不会重载正在编辑的内容。手动保存，关闭前必须确认“已保存”；没有自动保存、备份、跨窗口锁或云同步。多个窗口请顺序编辑。上限为 20000 个 UTF-16 码元，与 HTML maxlength 和 JS 字符串长度一致。用户文本只进入 textarea.value 或 textContent。

## 本机验收

- 嵌入窗口、分离窗口各打开一次，键盘可操作，小窗口可滚动。
- 首次拒绝读取时不能编辑，允许后点击“重试加载”；拒绝保存时输入保留。
- 输入中文、多行、`<script>alert(1)</script>`，保存后关闭插件、重启宿主再打开，内容应原样显示。
- 保存中不能再次编辑或提交。浅深主题下文本和焦点可见。

自动测试不代表以上原生验收已完成；维护者须记录宿主版本、安装包、实际步骤与结果，不用真实重要数据测试。

许可为 MIT，见 LICENSE。模板未接入商店索引，不自动发布或安装。
