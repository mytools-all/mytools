export type ToolStatus = "builtin" | "ready" | "planned";

export interface ToolReference {
  /** Rubick package name on npm. */
  package: string;
  /** Human label of the reference plugin. */
  label: string;
  /** npm page of the reference plugin. */
  url: string;
  /** Logo of the reference plugin. */
  logo?: string;
}

export interface ToolEntry {
  /** Our plugin package name (also the npm package name later). */
  name: string;
  title: string;
  description: string;
  category: string;
  version: string;
  status: ToolStatus;
  /** Host capabilities the tool needs. */
  capabilities: string[];
  /** Feature highlights shown in the card. */
  features: string[];
  /** npm spec used once the package is published. */
  npmSpec?: string;
  reference?: ToolReference;
}

export const TOOL_CATALOG: ToolEntry[] = [
  {
    name: "mytools-calc",
    title: "计算稿纸",
    description: "表达式求值 + 稿纸记录：算完留痕，随时复制结果。",
    category: "效率",
    version: "0.1.0",
    status: "ready",
    capabilities: ["clipboard.write", "storage.get", "storage.set"],
    features: ["递归下降表达式求值", "稿纸历史记录", "一键复制结果"],
    npmSpec: "@mytools-all/mytools-calc@0.1.0",
    reference: {
      package: "rubick-plugin-calc",
      label: "计算器（rubick-plugin-calc）",
      url: "https://www.npmjs.com/package/rubick-plugin-calc",
      logo: "https://pic1.zhimg.com/80/v2-dc75cd3de8e745d6b702fa5b0404d898_720w.png",
    },
  },
  {
    name: "mytools-markdown",
    title: "Markdown 编辑器",
    description: "左写右看：实时预览渲染，草稿自动保存。",
    category: "效率",
    version: "0.1.0",
    status: "ready",
    capabilities: [
      "storage.get",
      "storage.set",
      "clipboard.write",
      "shell.open",
      "fs.exporttext",
    ],
    features: ["实时预览", "标题/列表/引用/代码渲染", "草稿自动保存"],
    npmSpec: "@mytools-all/mytools-markdown@0.1.0",
    reference: {
      package: "rubick-markdown",
      label: "Markdown 编辑器（rubick-markdown）",
      url: "https://www.npmjs.com/package/rubick-markdown",
      logo: "https://pic1.zhimg.com/80/v2-bd54fa5f6b53a0d08df58889511b7656_720w.png",
    },
  },
  {
    name: "mytools-json",
    title: "JSON 编辑器",
    description: "JSON 格式化、压缩、查询和格式转换。",
    category: "开发",
    version: "0.1.0",
    status: "ready",
    capabilities: ["clipboard.write"],
    features: ["格式化", "压缩", "复制结果"],
    npmSpec: "@mytools-all/mytools-json@0.1.0",
    reference: {
      package: "rubick-json-editor",
      label: "JSON 编辑器（rubick-json-editor）",
      url: "https://www.npmjs.com/package/rubick-json-editor",
      logo: "https://picx.zhimg.com/80/v2-9829832ee97d87a27ba83995fae70759_720w.png",
    },
  },
  {
    name: "mytools-ipconfig",
    title: "ipconfig",
    description: "本机网卡与 IP 信息一览：IPv4、网关、MAC、DNS。",
    category: "系统",
    version: "0.1.0",
    status: "ready",
    capabilities: ["system.networkdetails", "system.netinterfaces", "clipboard.write", "net.fetch"],
    features: ["网卡列表", "IPv4 / IPv6 地址", "网关与 DNS"],
    npmSpec: "@mytools-all/mytools-ipconfig@0.1.0",
    reference: {
      package: "ip-config-rubick-plugin",
      label: "ip-config（ip-config-rubick-plugin）",
      url: "https://www.npmjs.com/package/ip-config-rubick-plugin",
      logo: "https://picx.zhimg.com/80/v2-984076df46c3a43a75834e26cc1c7dee_720w.png",
    },
  },
  {
    name: "mytools-color",
    title: "颜色助手",
    description: "颜色转换与配色：HEX / RGB / HSL 互转，色板与对比度检查。",
    category: "效率",
    version: "0.1.0",
    status: "ready",
    capabilities: ["clipboard.write", "storage.get", "storage.set"],
    features: ["HEX / RGB / HSL 互转", "取色板", "对比度检查"],
    npmSpec: "@mytools-all/mytools-color@0.1.0",
    reference: {
      package: "rubick-color",
      label: "颜色助手（rubick-color）",
      url: "https://www.npmjs.com/package/rubick-color",
      logo: "https://pic1.zhimg.com/80/v2-7f3076f6125748df6757141e93e458ad_720w.png",
    },
  },
  {
    name: "mytools-color-picker",
    title: "取色器",
    description: "屏幕取色：跟随鼠标读取像素颜色，复制 HEX / RGB。",
    category: "效率",
    version: "0.1.0",
    status: "ready",
    capabilities: [
      "clipboard.write",
      "storage.get",
      "storage.set",
      "screen.pickcolor",
    ],
    features: ["跟随鼠标取色", "放大镜预览", "复制 HEX / RGB"],
    npmSpec: "@mytools-all/mytools-color-picker@0.1.0",
    reference: {
      package: "rubick-system-color-picker",
      label: "取色器（rubick-system-color-picker）",
      url: "https://www.npmjs.com/package/rubick-system-color-picker",
      logo: "https://pic1.zhimg.com/80/v2-5f1810a71af6eefcd77edbbf07ea1cc7_720w.png",
    },
  },
  {
    name: "mytools-clipboard",
    title: "系统剪贴板",
    description: "剪贴板历史：浏览、复制、删除之前的复制内容。",
    category: "系统",
    version: "0.1.0",
    status: "ready",
    capabilities: ["clipboard.history", "clipboard.write", "clipboard.remove"],
    features: ["历史列表", "搜索过滤", "复制与删除"],
    npmSpec: "@mytools-all/mytools-clipboard@0.1.0",
    reference: {
      package: "rubick-system-clipboard",
      label: "系统剪贴板（rubick-system-clipboard）",
      url: "https://www.npmjs.com/package/rubick-system-clipboard",
      logo: "https://pic1.zhimg.com/80/v2-8808dd420ee9804ad588aaee01bd6816e_720w.png",
    },
  },
  {
    name: "mytools-search",
    title: "本地搜索",
    description:
      "使用 Everything 索引搜索全盘文件与文件夹，支持原生搜索语法和预览。",
    category: "文件",
    version: "0.1.0",
    status: "ready",
    capabilities: [
      "fs.searchstatus",
      "fs.globalsearch",
      "fs.searchpreview",
      "shell.open",
      "shell.reveal",
      "clipboard.write",
      "app.list",
      "app.launch",
    ],
    features: ["Everything 索引", "分类与快捷键", "文件预览与定位"],
    npmSpec: "@mytools-all/mytools-search@0.1.0",
    reference: {
      package: "utools-searchmaster",
      label: "uTools 文件搜索（Everything）",
      url: "https://www.u-tools.cn/plugins/detail/搜索大师/",
      logo: "https://picx.zhimg.com/80/v2-8a5188bd51bf5881a7c509436ca4523c_720w.png",
    },
  },
  {
    name: "mytools-rename",
    title: "文件批量重命名",
    description: "批量改名：规则预览、冲突检测、一次确认。",
    category: "文件",
    version: "0.1.0",
    status: "ready",
    capabilities: ["fs.importdropped", 
      "fs.pickdirectory",
      "fs.pickfiles",
      "fs.search",
      "fs.renamemany",
      "clipboard.write",
    ],
    features: ["批量规则预览", "冲突检测", "撤销清单"],
    npmSpec: "@mytools-all/mytools-rename@0.1.0",
    reference: {
      package: "utools-batch-rename",
      label: "文件批量重命名（rubick-plugin-rename）",
      url: "https://www.u-tools.cn/plugins/topic/74/",
      logo: "https://pic1.zhimg.com/80/v2-959666589c798bf2a259eb7068e9e201_720w.png",
    },
  },
  {
    name: "mytools-ctool",
    title: "Ctool",
    description: "开发小工具集：编码转换、时间戳、哈希、正则、URL 解析。",
    category: "开发",
    version: "0.1.0",
    status: "ready",
    capabilities: ["clipboard.write"],
    features: ["Base64 / URL 编解码", "时间戳与哈希", "正则与 JSON 工具"],
    npmSpec: "@mytools-all/mytools-ctool@0.1.0",
    reference: {
      package: "rubick-ctool",
      label: "Ctool（rubick-ctool）",
      url: "https://www.npmjs.com/package/rubick-ctool",
      logo: "https://static.91jkys.com/attachment/kaer-admin/8fafac108cfc486180dedc136df20b76707add249f639eec70c10a25cd129605.png",
    },
  },
  {
    name: "mytools-ocr",
    title: "讯飞 OCR",
    description: "图片识别为文字：调用讯飞开放平台 OCR 接口（需配置密钥）。",
    category: "效率",
    version: "0.1.0",
    status: "ready",
    capabilities: [
      "net.fetch",
      "storage.get",
      "storage.set",
      "storage.del",
      "clipboard.write",
    ],
    features: ["图片拖拽识别", "识别结果编辑", "密钥本地保存"],
    npmSpec: "@mytools-all/mytools-ocr@0.1.0",
    reference: {
      package: "rubick-xunfei-ocr",
      label: "讯飞 OCR（rubick-xunfei-ocr）",
      url: "https://www.npmjs.com/package/rubick-xunfei-ocr",
      logo: "https://picx.zhimg.com/80/v2-4c056cc6ca01ba1472ce200770e73ffa_720w.png",
    },
  },
];

export const TOOL_CATEGORIES = Array.from(
  new Set(TOOL_CATALOG.map((entry) => entry.category)),
);
