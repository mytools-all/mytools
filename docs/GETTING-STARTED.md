# 新手入门：从使用插件到理解宿主

状态：教学路线，适用 MyTools 0.1.0 / 插件 API v1，整理于 2026-11-18。本文不要求先掌握 Rust；宿主编译环境以根目录 README 为准。示例未发布到 npm。

## 1. 先知道自己要做什么

| 目标 | 先学什么 | 是否需要 Rust |
|---|---|---|
| 使用现有工具 | 安装、搜索、权限和插件中心 | 不需要 |
| 开发插件 | HTML、CSS、JavaScript、JSON、异步与错误处理 | 普通插件不需要 |
| 修改宿主界面 | TypeScript、Vue 3、Pinia | 仅界面改动通常不需要 |
| 增加宿主系统能力 | 插件契约、Rust、Tauri、权限与测试 | 需要 |

完全没有编程经验时，先练习制作一个网页：标题、文本框、按钮；点击按钮读取文本并显示，再练习捕获一个失败的异步操作。能解释事件、对象、函数、Promise 和 try/catch 后，再进入插件模板。不要同时学习完整 Vue 和 Rust 才开始第一个插件。

## 2. 三类仓库和四种目录

推荐工作区结构如下，顶层 MyTools 是普通文件夹，每个项目自己管理 Git：

```text
MyTools/
├── mytools/                   宿主仓库，含本教程
├── mytools-home/              商店索引仓库
│   └── plugins/database.json  商店的包名、版本、介绍
└── plugins/
    ├── mytools-demo/          独立插件仓库
    ├── mytools-calendar/      独立插件仓库
    └── mytools-notes-starter/ 从模板复制的练习目录
```

GitHub 保存源码和提交历史；npm 保存可安装的插件包；database.json 是商店目录，不包含插件实现，也不会自动授予权限。宿主仓库可以单独编译，不要求这个布局。旧资料中的 app 指的是这里的 mytools；旧资料将插件放入索引仓库内，本路线采用上面的同级布局。

分清四个位置：源码目录供修改；npm pack 生成的 tgz 是交付包；应用的插件根目录用于实际运行；AppData 中的插件存储用于保存用户数据。把源码复制进工作区不代表已经安装，重新安装包也不代表清空了旧数据。

## 3. 先完成一次真实使用

1. 按[根 README](../README.md)下载并启动正式版。默认可能隐藏在托盘，可用 Alt+Space 或托盘“主页”打开。
2. 在插件商店安装 Demo，从主页搜索并打开它。
3. 使用自己的测试文本尝试复制操作，观察宿主的权限提示；拒绝时应显示失败。
4. 在插件中心的开发者页面找到“安装插件包（.zip / .tgz）”“打开插件目录”和“重新扫描”。
5. 打开一次分离窗口，理解插件页与宿主窗口的区别。

完成标志：能说清“我安装了哪个包、插件在哪里运行、谁负责询问权限”。此阶段无需构建宿主。

## 4. 从便签模板完成开发闭环

开发测试需要 Node.js 24+ 和 npm；普通插件无需 pnpm、Rust、MSYS2。先检查 `node --version` 和 `npm --version`，缺少 npm 时修复 Node 安装或 PATH。模板没有第三方依赖，不需要 npm install。

在 MyTools 工作区根目录执行，目标目录必须不存在：

```powershell
Copy-Item -Recurse ./mytools/docs/plugin-development/templates/mytools-notes ./plugins/mytools-notes-starter
cd ./plugins/mytools-notes-starter
npm test
npm pack --dry-run
npm pack
```

如果只克隆了宿主，在宿主根目录使用 `Copy-Item -Recurse ./docs/plugin-development/templates/mytools-notes ../mytools-notes-starter`，然后进入该目录执行同样的测试和打包命令。

阅读[模板说明](plugin-development/templates/mytools-notes/README.md)，在开发者页面安装生成的 tgz，搜索“便签入门”。输入一段文字，保存，确认成功，再关闭插件、重启宿主、重新打开。不要用重要资料做测试。

接着只改标题与样式，重新测试、打包、安装、关闭重开，确认新版本文件已加载。最后练习修改模型逻辑。插件运行时没有 Node.js，Node 只用于开发测试和打包。

完成标志：你能从修改源码走到实际安装验证，并能解释以下文件：

| 文件 | 为什么需要 |
|---|---|
| plugin.json | 宿主用它识别身份、入口、搜索词与所需能力 |
| package.json | npm 用它描述包、版本、交付文件与测试命令 |
| index.html / style.css | 网页结构与外观 |
| app.mjs | 事件、DOM 与宿主桥接 |
| model.mjs | 状态和存储流程，可脱离网页测试 |
| tests/ | 验证数据保护和失败路径，不放进交付包 |

## 5. 理解一次插件调用

```text
点击“保存”
→ app.mjs 处理事件
→ model.mjs 冻结编辑，生成 {version:1,text}
→ window.mytools.call('storage.set', {key:'notes.v1', value:...})
→ 宿主桥接绑定插件身份和会话
→ Rust 运行时检查身份、能力与授权
→ 存储服务写入数据
→ 成功或失败返回插件
→ 界面更新“已保存”或保留输入供重试
```

iframe 是装插件网页的容器；桥接是网页请求宿主能力的入口；IPC 是前端与 Rust 之间的消息通道。开发普通插件只使用 window.mytools，不直接调用 Tauri，不自己发送内部协议消息。

API 名、参数和返回值需要查[插件指南](plugin-development/README.md)及[能力参考](04-plugin-spec.md)。不在文档中的能力先向维护者确认，不能猜一个名称调用。浏览器预览没有桥接；测试中的模拟存储也不等于真实宿主。

便签的关键不是一个文本框，而是：读取失败时不覆盖旧数据；未知格式停止写入；保存失败保留输入；保存成功才更新成功状态；重复就绪事件不丢失编辑。模板刻意只提供手动保存，不保证关闭前未保存内容、跨窗口并发编辑或云同步。

## 6. 再理解宿主

先读[项目上下文](PROJECT-CONTEXT.md)，按职责认识目录：

| 目录 | 职责 | 先看什么 |
|---|---|---|
| src/views、components、stores | Vue 界面与 Pinia 状态 | 找一个实际使用过的页面 |
| src/runtime、composables/pluginBridge.ts | 插件生命周期与桥接 | 搜索 storage.set 的调用链 |
| src-tauri/src/commands | Tauri 消息入口 | commands/capability.rs |
| src-tauri/src/runtime | 身份、会话、能力调度 | 跟随 dispatch 的定义 |
| src-tauri/src/services | 业务与存储 | 沿调用定位对应 handler |
| src-tauri/src/infrastructure | Windows、路径等适配 | 需要系统功能时再读 |
| src-tauri/crates/mytools-core | 不依赖 Tauri 的规则 | 清单、权限与测试 |

在编辑器用“转到定义”和全局搜索沿一条调用阅读。先找输入、校验、输出、错误四件事；暂时跳过不相关的托盘、OCR、热键和构建逻辑。前端进入命令层后仍要经过运行时校验，命令层不应变成插件业务实现。

## 7. Rust 基础路线

用独立练习目录学习，不要在宿主内尝试会破坏编译的语法实验。宿主当前采用 Windows GNU 工具链，不能照通用教程把项目切换成 MSVC；完整环境安装与构建遵循根 README。

| 顺序 | 概念 | 用自己的话理解 | 小练习 |
|---|---|---|---|
| 1 | Cargo、crate、mod、use | Cargo 管构建；crate 是编译单元；模块组织代码 | 新建独立练习包并 cargo run / cargo test |
| 2 | let、mut、函数、struct | 默认不可变；struct 描述一组字段 | 定义便签结构并修改文本 |
| 3 | enum、match、Option | 明确列出状态；没有值用 None 表达 | 区分“尚无便签”和“空文本便签” |
| 4 | String、&str、Vec | 拥有的文本、借用的文本视图、可增长列表 | 写接收 &str 的字符计数函数 |
| 5 | 所有权、借用、&、&mut | 值有所有者；引用借用它；可变借用受限制 | 制造一次 moved value 报错，再用借用修复 |
| 6 | Result、?、错误类型 | 成功和失败都在返回类型中；? 提前传递错误 | 读取不存在的文件并显示错误，不 unwrap |
| 7 | Serde、JSON | 在 Rust 类型与插件传来的 JSON 间转换 | 序列化便签并拒绝错误版本 |
| 8 | async/await、Arc、锁 | 等待操作与共享状态；Arc 不自动让内部值可变 | 阅读项目已有异步函数和共享状态测试 |
| 9 | trait、泛型、生命周期 | 公共行为、可复用类型、引用有效关系 | 遇到相关签名再查，不急着重写架构 |

Rust 的类型标注不是 JSON。下面的 `&str` 表示借用文本，`Result` 列出成功或失败类型，`match` 处理两种结果：

```rust
fn count_note(text: &str) -> Result<usize, &'static str> {
    let count = text.chars().count();
    if count > 100 {
        return Err("练习限制：最多 100 个 Unicode 标量值");
    }
    Ok(count)
}

fn main() {
    let text = String::from("我的便签");
    match count_note(&text) {
        Ok(count) => println!("计数：{count}"),
        Err(message) => eprintln!("{message}"),
    }
    println!("借用结束后仍可使用：{text}");
}
```

这是独立语法练习，不是插件 API。Rust 的 chars().count() 与 JavaScript 的字符串 length 计数不同，也不等同于用户感知字符数；实际跨语言契约应明确单位。

推荐阅读 [Rust 官方教材](https://doc.rust-lang.org/book/)，先学第 1–6 章，再看模块、错误处理、泛型与 trait、测试、智能指针和并发；所有权与借用必须亲手运行示例。教材提供不同语言版本的入口。本项目维护还需要结合 Tauri 和 Serde 的实际用法，语言入门不能替代权限与业务规则学习。

## 8. 常见卡点与下一步

| 现象 | 先检查 |
|---|---|
| window.mytools 不存在 | 是否只在浏览器打开；是否安装到宿主运行 |
| 改了源码但界面没变 | 是否重新打包安装并关闭重开 |
| 读取失败后输入框禁用 | 这是保护旧数据；处理权限或存储错误后重试 |
| 保存按钮一直不可用 | 是否还在加载、保存中，或内容没有变化 |
| 源码在 plugins 下却找不到插件 | 工作区不是实际插件安装根 |
| Rust 链接或 windres 报错 | 按根 README 检查 GNU 工具链和 PATH |

下一步读[完整插件指南](plugin-development/README.md)，用[模型独立开发验收](plugin-development/model-evaluation.md)的检查项审视自己的插件。阅读历史文档时先看状态头；不把历史测试数字和未验收事项当作本机当前结果。
