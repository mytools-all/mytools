# MyTools 图标第一版

使用内置 image_gen 生成。PNG 图稿：mytools-icon-v1.png。

设计：深靛蓝圆角底，模块化 M 表达 MyTools 与工具组合，青色及暖黄色作为功能模块点缀。外部背景透明。

应用资源由 Tauri CLI 从本图生成，位于 `src-tauri/icons`；主界面与悬浮球引用其中的 `128x128.png`，原生窗口、托盘及安装包读取应用默认图标，网页标签使用 `public/mytools.png`。ICO 包含 16、24、32、48、64、256 像素图层。

`src-tauri/build.rs` 显式指定 Windows ICO，并监听 `icons` 目录变化，以便图标更新后重新生成 EXE 内嵌资源。验收时从新 EXE 的临时副本提取系统关联图标，避免仅验证前端图片或命中旧路径缓存。

程序默认原生图标与分离窗口图标由 `infrastructure/app_identity.rs` 直接从 `128x128.png` 构造并缓存。插件窗口和插件中心窗口在创建与复用时均显式设置该图标。窗口验收还需分别读取两类新窗口的原生图标。

Windows 标题栏小图标与任务栏大图标是两个独立的窗口属性。`app_identity::apply` 设置小图标，并以 `LR_SHARED` 从当前 EXE 的 32512 图标资源取得模块生命周期的大图标，通过 `WM_SETICON / ICON_BIG` 设置任务栏图标。窗口完成图标设置后才显示。验收分别读取 `WM_GETICON / ICON_SMALL` 与 `ICON_BIG`，大图标缺失时不得回退到小图标。

Windows 任务栏分组还需独立指定 `AppUserModel.ID`、`RelaunchIconResource`、`RelaunchCommand` 和显示名称。`taskbar_identity` 在窗口显示前设置这些属性；图标文件保存在应用数据 `branding` 目录，文件名由 ICO 内容生成，图标变化时提供新的缓存地址。属性字符串统一使用 Windows `VT_LPWSTR`。验收必须检查窗口分组属性，并确认用户实际看到的任务栏显示，窗口内图标不能代替此项。

最终提示词：

Design ONE original, simple, polished app icon for MyTools, a Windows desktop productivity launcher and modular plugin toolbox. Deliver only the finished standalone icon, large and centered on a truly transparent canvas, no mockup, no presentation sheet, no labels, no wordmark. A deep midnight-indigo rounded-square tile with a distinctive bold geometric capital M mark assembled from three or four clean interlocking rounded modules, suggesting tools coming together and a fast launch. Mostly bright ivory M with a single electric cyan module and a restrained tiny warm amber accent integrated coherently into the geometry. Flat vector-like execution, crisp edges, generous negative space, consistent substantial stroke widths, extremely readable at 24 and 32 pixels. The M must be visually obvious. Subtle tile gradient is acceptable but no 3D, no glossy bevel, no texture, no orbit, no chain rings, no linked circles, no gear, no tiny decorative details, no Tauri logo resemblance. Icon fills around 85 percent of the square canvas with transparent area outside the rounded tile. Professional, calm, memorable, original.
