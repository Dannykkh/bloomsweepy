# BroomSweepy

<p align="center">
  <img src="apps/desktop/src-tauri/icons/app-icon-master.png" width="112" alt="BroomSweepy 扫帚应用图标">
</p>

<p align="center">
  <a href="README.en.md">English</a> |
  <a href="README.md">한국어</a> |
  <a href="README.ja.md">日本語</a> |
  <strong>简体中文</strong>
</p>

BroomSweepy 是一款适用于 Windows 和 macOS 的存储分析与清理审核工具。它可以查找大文件、经过内容验证的重复文件、空文件夹、清理候选项、文件名，以及受支持文档中的文字。实际扫描由本地 Rust 引擎执行，AI 连接是可选功能。

无需为不同语言安装不同版本。首次启动默认显示英语。在 `Settings > Display language` 中选择 English、한국어、日本語 或简体中文后，应用界面、Windows 托盘菜单和向 AI 请求的回复语言会立即切换，并且仅保存在此电脑上。

## 快速开始

1. 打开`存储空间`并选择要检查的文件夹。
2. 选择`创建存储空间地图`。矩形越大，所占空间越多；选择文件夹矩形可以继续进入下一层。
3. 仅在需要详细结果时运行`查找大文件和重复文件`。BroomSweepy 只会读取重复候选文件的内容。
4. 逐项检查所选内容，并在最终确认后将其移动到操作系统回收站。

按名称和位置查找时使用`查找文件`，按正文查找时使用`搜索文档`。这些基本功能无需连接 AI、CLI 或 MCP。

## 界面预览

![BroomSweepy 简体中文设置界面](docs/assets/screenshots/v1.4.0-settings-zh-CN.png)

语言选择只会更改现有应用的显示内容，不会安装另一个版本，也不会更改操作系统设置。

## v1.4.0 主要更新

- 一个应用内完整提供英语、韩语、日语和简体中文，并以英语作为默认显示语言。
- 所选语言同时应用于 HTML 语言信息、Windows 托盘菜单，以及发送给已安装 AI CLI 的回复语言要求。
- 加粗扫帚轮廓，使任务栏、托盘、Dock 和安装程序中的 16～32px 小图标更容易辨认。
- Windows 安装程序内置 `bloomsweepy-mcp`，用户可在设置界面自行注册或移除 Codex 与 Claude Code 连接。
- 外部 AI 只接收不含路径且有数量限制的摘要；准确路径与最终确认始终留在 BroomSweepy 内。
- 窗口标题显示软件包版本，便于确认当前运行的构建。

## 主要功能

- 在仪表板中查看各驱动器用量、可用空间、最近的回收站操作和最近新增文件。
- 可逐层进入文件夹的比例矩形`存储空间树状图`。
- 依次使用大小分组、部分 BLAKE3、完整 BLAKE3 和最终字节比较来确认重复文件。
- 使用本地 SQLite FTS 快速搜索文件名、路径和受支持文档的正文。
- 支持索引 TXT、Markdown、源代码、PDF 文本层、DOCX、XLSX、PPTX 和 HWPX。
- 通过只读注册表信息列出 Windows 安装应用，通过 `.app/Contents/Info.plist` 列出 macOS 应用。
- 审核 Temp、缓存、AppData 和卸载残留候选项，不自动删除注册表内容。
- 后台扫描、内存与结果数量上限、取消检查点，以及执行前重新验证。
- 使用同步 JSONL 日志记录操作，移动到操作系统回收站，并审核中断的操作。
- 可选的 Docker 专用界面；Docker 卷永远不会被清理。
- 开发分支（发布前验证中）：由用户明确开关的 Windows/macOS 登录时启动，以及只读的内存总量、可用量、已用量和 `sysinfo` 所报告的交换空间指标。Windows 交换空间数值是基于提交量的估算值，并非页面文件的当前使用量；该面板不会清理缓存或内存泄漏。

## AI、CLI 与 MCP 的边界

扫描、索引、重新验证和移动到回收站都由本机上的 BroomSweepy 执行。Codex、Claude Code、Grok、Antigravity 或 Ollama 可以概括受限结果并建议审核顺序，但应用不会向它们提供不受限制的文件系统工具。

MCP 清理工具只公开匿名候选编号和有上限的摘要，不提供批准、永久删除、执行回收站移动、写入注册表或清空回收站的工具。准确路径只在应用内显示，并且只有用户在应用中确认后才会执行清理。

## Docker 是可选功能

Docker 功能默认关闭。关闭时，BroomSweepy 不会查找或运行 Docker CLI。启用后，独立的 Docker 界面可以读取 `docker system df`，并且只预览针对旧构建缓存、悬空镜像和已停止容器的固定白名单命令。Docker 清理不会经过操作系统回收站，因此需要单独确认该操作无法恢复。

## 平台与开发

Windows 安装程序必须在 Windows 上构建。经过签名和公证的 `.dmg` 必须在 macOS 上构建。

开发环境需要 Rust stable、Node.js 22 或更高版本、npm；Windows 还需要 WebView2 和 MSVC Build Tools。

```powershell
cd apps/desktop
npm install
npm run tauri dev
```

有关详细实现和安全边界，请参阅[韩语详细 README](README.md)、[CLI 控制](docs/cli-control.md)、[跨平台架构](docs/architecture/cross-platform-desktop.md)、[自动启动与系统内存状态](docs/architecture/startup-memory-status.md)和[安全回收站操作](docs/architecture/safe-trash-actions.md)。

## 重要提示：数据丢失与恢复责任

BroomSweepy 设计为只处理用户选择并最终确认的项目。但是，是否能够恢复仍可能受到操作系统权限、回收站设置、同步服务，以及外置或网络驱动器状态的影响。Docker 清理不使用操作系统回收站，已完成的步骤无法恢复。

执行清理前，请备份重要数据，并核对所有选定路径、文件和 Docker 分类。除法律规定不得排除的责任外，项目提供者和贡献者不对用户发起的文件移动、删除、清空回收站或 Docker 清理造成的数据丢失或恢复失败承担责任。
