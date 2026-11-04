# MyTools 插件开发指南

状态：公开 API v1 的可执行开发指南，核对日期 2026-11-04。读者只需要本目录、[API 参考](../04-plugin-spec.md)与 [MyTools Demo](https://github.com/mytools-all/mytools-demo) 源码，不需要宿主源码或历史聊天。

## 1. 最小交付物

插件是独立静态网页包，可以用原生 HTML/CSS/JS，也可以构建后交付静态资源。运行时没有 Node.js / Electron / require，也不能直接访问 Tauri、宿主源码、其他插件目录。所有资源使用相对路径，包内自带依赖，不运行时加载 CDN。

```text
mytools-example/
  plugin.json
  package.json
  index.html
  app.mjs
  style.css
  icon.svg
  LICENSE
  README.md
  vendor/                 # 有第三方依赖时连同许可证交付
  tests/                  # 不进入发布包
```

plugin.json：

```json
{
  "name": "mytools-example",
  "pluginName": "示例工具",
  "description": "一句话描述",
  "version": "0.1.0",
  "author": "MyTools",
  "main": "index.html",
  "icon": "icon.svg",
  "apiVersion": 1,
  "capabilities": ["storage.get", "storage.set"],
  "features": [{"code":"main","explain":"打开示例工具","cmds":["示例工具","example"]}]
}
```

package.json：

```json
{
  "name": "@mytools-all/mytools-example",
  "version": "0.1.0",
  "type": "module",
  "main": "index.html",
  "license": "MIT",
  "files": ["plugin.json","index.html","*.mjs","*.css","icon.svg","vendor/","LICENSE","THIRD_PARTY.md","README.md"],
  "scripts": {"test":"node --test tests/*.test.mjs"},
  "publishConfig": {"access":"public"}
}
```

运行身份是不带作用域的 name；npm 名称可带作用域，两个文件版本一致。features[].code 唯一且稳定，cmds 是启动搜索别名，不是内部搜索框。只声明实际使用的能力；空列表存在旧版兼容行为，不能用来表示显式拒绝所有权限。

## 2. 初始化与宿主调用

宿主在页面脚本前注入 window.mytools；插件不要自行加载桥接脚本、发送协议消息或编造身份和会话。

```js
const status = document.querySelector('#status');
function report(error) {
  status.textContent = error?.message || String(error);
}
async function call(id, args = {}) {
  if (!window.mytools) throw new Error('请从 MyTools 打开此插件');
  return window.mytools.call(id, args);
}
if (window.mytools) {
  window.mytools.onPluginReady(() => { void initialize().catch(report); });
} else {
  status.textContent = '网页预览：宿主存储和系统功能不可用';
}
```

initialize 需要幂等，避免重复 ready 覆盖正在编辑的数据。异步 DOM/生命周期回调必须 catch，显示可理解的失败状态。当前 iframe 的失败是 Error(message)，不要依赖 error.code。宿主负责询问能力授权、记录授权并重试一次；拒绝后插件显示失败，不能自动授予权限。需要确认的插件业务操作用网页内联 UI 或 HTML dialog，不调用 alert/confirm/prompt，也不申请 Tauri dialog ACL。

## 3. 生命周期与窗口

| API | 用法 |
|---|---|
| onPluginReady(callback) | 桥接就绪；载入存储，晚注册会补发 |
| onPluginEnter(callback) | 进入时收到 env.code/type/payload/plugin/apiVersion |
| getEnv() | 最近一次环境，尚未就绪时为默认值 |
| onPluginOut(callback) | 离开通知，用于结束计时/订阅；不能依赖此时才异步保存 |
| onSearch(callback) | 接收 {query,key?}；key 可表示 Enter / 方向键 |
| onFilesDropped(callback) | 系统拖放通知，路径仍需 fs.importdropped 获取 |

env 还有可选布尔 hostSearch、launchContext。插件 iframe 可能被销毁，返回后重新创建；持久化必须在每次修改时完成，不能仅在退出时保存。

清单可设置 `"hostSearch":"搜索待办…"`。ready 的 env.hostSearch===true 时隐藏自己的输入，通过 onSearch 更新同一个查询；false 时保留自己的输入。分离窗口没有公共搜索框。无需搜索的插件不声明 hostSearch。

插件同时适应主窗口小尺寸、独立窗口和窄屏；用 minmax(0,1fr)、min-width:0、明确滚动容器，断点下改为上下布局，不依赖宿主固定高度。支持系统浅深主题、键盘操作、焦点样式与可读的按钮名。

## 4. 持久化契约

```js
const saved = await call('storage.get', {key:'state.v1'});
await call('storage.set', {key:'state.v1', value:{version:1, items:[]}});
```

get 返回原始 JSON 值，键不存在为 null；set 成功返回 null；del 返回 boolean。value 只存 JSON 数据，不能是 DOM、Date 实例或函数。不要传 namespace，默认按插件身份取键。不要使用 localStorage/IndexedDB 保存用户数据，回环端口变化会导致数据不可见。

加载完成前禁止修改，读取失败显示“重试”，不能拿空数据覆盖原数据。对保存结构进行版本和字段校验，未知版本/损坏数据应停止写入并提示，不能静默重置。写入串行，防止旧请求覆盖新状态；成功后再显示“已保存”，失败保留输入并允许重试。简单表单可保存期间禁用修改，保存成功再提交内存状态。多个窗口共享存储但目前没有 CAS/订阅保证；需要解决并发编辑的插件须自行定义策略，不能宣称多端同步。

日期键用显式 YYYY-MM-DD；日历选择的日期是民用日期，不要用 toISOString() 把本地零点转换成前一天。用户文字通过 textContent 或转义渲染。

## 5. 文件、网络与面板

详细参数见 [能力表](../04-plugin-spec.md#42-首版能力目录)。只通过 call 使用宿主功能：

- 导出文本：fs.exporttext({name,content})，返回 {path} 或 {cancelled:true}；不要依赖 iframe 的 a.download 来替代系统保存。
- 文件选择：fs.pickfiles({directories:false}) → {entries}，条目含 path/name/directory/size/modified。文件夹可用 directories:true。
- 拖入：onFilesDropped 后调用 fs.importdropped({}) → {entries}，60 秒内仅一次。声明两个文件相关能力时也不能自行传入路径绕过授权。禁止拖入图标需检查宿主原生拖放及 Windows 进程权限，不能只监听 DOM drop。
- 网络：net.fetch({url,method,headers,body}) → {status,headers,body}，body 是文本，按需 JSON.parse；申请权限，明确超时/失败状态。
- 面板：声明 context.read，ready 中仅当 env.launchContext 才调用 {types:['files']} 或 ['image']/['text']，返回 null 或快照。文件位于 context.files，图片是 context.image.dataUrl。一次消费、120 秒到期；只加载预览，不自动执行外部动作。[完整边界](../13-launch-context.md)。

本地数据处理无需申请网络、剪贴板或文件权限。第三方库属于插件私有实现，不因引入库而新增宿主能力。

## 6. 依赖与离线资源

固定依赖版本，记录来源、许可证、版本和 SHA-256。无构建插件可用 npm pack <包名>@<固定版本> 下载，再将所需浏览器产物及 LICENSE 放入 vendor；别只在 package.json 写 dependencies，宿主不替插件执行 npm install。UMD 脚本在模块脚本之前加载，Node 测试可用 node:vm 在新上下文载入同一份 UMD 文件；测试浏览器和 Node 使用的是同一产物。

例如日历可选 [lunar-javascript](https://github.com/6tail/lunar-javascript)：Solar.fromYmd(y,m,d).getLunar() 取得农历；Lunar.fromYmd(y,m,d).getSolar() 转公历；闰月以负月份传入。LunarYear.fromYear(y).getMonthsInYear() 列出该农历年的月份，各月 getMonth()/getDayCount()；月份中文由该月初一 getMonthInChinese() 取得。节日使用 solar.getFestivals()/lunar.getFestivals()，节气使用 lunar.getJieQi()。校验真实日期和支持年份，不允许 Date 自动把 2 月 30 日滚到 3 月。节日日期不等于年度放假/补班，缺少官方年度数据不显示“休/班”。第三方 API 以固定版本文档和测试为准，不是宿主 API。

## 7. 测试、打包和本地安装

```powershell
npm test
npm pack --dry-run
npm pack
```

检查归档包含 plugin.json 的入口、脚本、样式、图标、vendor 和许可证，排除测试、node_modules、凭据与运行数据。在 MyTools → 插件中心 → 开发者，用“安装插件包（.zip / .tgz）”选择生成的 tgz。目录开发时点击“打开插件目录”，将完整插件目录放入其中，再点击“重新扫描”。宿主复制到统一插件根；修改源码后需重新安装，不是自动热重载。重新打开插件测试实际安装包，而非只检查源码。

纯逻辑测试用 Node 内建 node:test。DOM 测试可以固定 happy-dom 作为开发依赖，模拟 window.mytools.call 和生命周期，检查首次载入、权限拒绝、读取失败重试、保存失败、重新打开持久化、文本转义及空状态。模拟桥接只用于测试，不随包启用。原生验收覆盖嵌入/分离、授权、重启持久化、窄窗口与深浅主题；测试数据不要写入现有用户数据。

插件无需 npm 发布即可完整运行。本地安装验证不需要新宿主命令或重新构建宿主。GitHub/npm 发布是后续独立步骤，先测试、检查包内容，再获得发布授权。

## 8. 商店条目与交付

mytools-home/plugins/database.json 中添加 schemaVersion 2 的条目，至少遵循现有 name/npm/version/published/title/description/category/author/license/repository/features/keywords/notes/capabilities 字段格式。未发布标记 published:false；不要把未创建的远程仓库说成已创建。宿主离线快照通过 pnpm catalog:sync 同步，不单独编辑分类。索引记录不会替代 plugin.json 或自动安装本地源码。

交付包含 README（功能、安装、权限、数据与限制）、LICENSE、测试和实际验证记录。对文档驱动开发，先记录所用文档版本，再按文档实现；发现缺项先修文档，重新验证。结果见 [日历验证记录](validation-calendar.md)。

离线整理尚未推送的本地索引时，可在宿主目录执行 `pnpm catalog:sync --file ../plugins/database.json`，与远程同步使用相同的字段校验；无参数仍读取远程索引。
