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

BroomSweepy 是一款适用于 Windows 和 macOS 的存储分析与清理审核工具。**Rust + Tauri 2 + React 是主项目**，延续了原 SwiftUI 应用宽敞的卡片、清晰的图标和玻璃质感界面。`BroomSweepy/` 中的 Swift 源码作为旧版参考实现保留。

大文件分析、经过验证的重复文件和文档搜索由本地 Rust 引擎处理，AI 连接是可选功能。一个安装版本支持英语、韩语、日语和简体中文，首次启动默认英语，可在 `Settings > Display language` 中更改。

## 快速开始

1. 在仪表板选择驱动器，或在`空间清理`中选择本地文件夹。选择文件夹后会生成空间地图。
2. 从最大的矩形开始检查，点击文件夹可进入下一层。项目菜单可以显示位置，或开始审核单个文件的回收站移动操作。
3. 一次执行大文件与重复文件检查，再自行选择需要审核的项目。实际移动前必须在应用中最终确认。
4. 使用`性能`查看 CPU 和内存，使用`文件管理`搜索名称和正文，使用`AI 助手`进行自然语言提问。

## 界面预览

这些截图使用当前 Rust 应用的实际 React 组件和公开演示数据。驱动器、文件、数值和对话均为示例，并非真实 AI 回复或用户文件。浏览器截图不包含 macOS 原生窗口材质效果。

### 多驱动器仪表板

![多驱动器仪表板](docs/assets/screenshots/v1.6.0-dashboard-zh-CN.png)

### 存储空间树状图

![存储空间树状图](docs/assets/screenshots/v1.6.0-overview-zh-CN.png)

### CPU 与内存

![CPU 与内存](docs/assets/screenshots/v1.6.0-performance-zh-CN.png)

### AI 助手

![AI 助手](docs/assets/screenshots/v1.6.0-assistant-zh-CN.png)

### 设置与显示语言

![设置与显示语言](docs/assets/screenshots/v1.6.0-settings-zh-CN.png)

## v1.6.0 主要更新

- 深色玻璃质感界面、大尺寸操作按钮、清晰图标和简化的导航。
- 当前驱动器显示为大卡片，其他驱动器显示为小卡片；点击后以动画交换位置并缩放。
- CPU 与内存环形图、主要应用用量、采样间平滑过渡，以及减少动态效果支持。
- macOS 应用内存清理仅将 BroomSweepy 主进程中未使用的 allocator 页面归还给系统，归还 0 字节也是正常完成。
- 树状图导航与单文件操作、文件身份及路径重新验证、回收站操作日志。
- 区分 AI CLI 未安装、无法执行、版本不兼容和需要登录的状态，改进对话、取消和历史记录处理。

## 安全与云端排除

扫描不会修改文件。大文件及重复检查、驱动器汇总、树状图、文件目录和文档索引会排除已知云同步根目录与仅在线项目。macOS 的 `~/Library/CloudStorage`、`~/Library/Mobile Documents` 以及可识别的 Google Drive、iCloud、OneDrive、Dropbox 路径在遍历前就会被排除，直接选择也不会扫描。已下载但位于识别出的云目录内的文件同样排除。无法保证识别任意迁移位置的同步文件夹或所有提供商。

重复文件依次通过大小、部分及完整 BLAKE3、最终字节比较验证。文件移动须经选择、重新验证、最终确认和日志记录后进入系统回收站。空文件夹查找为只读，不提供一般永久删除、清空回收站或自动删除注册表的操作。移入回收站的逻辑大小不等于新增可用空间。

内存清理不会清理系统整体 RAM、其他应用、WebView 辅助进程、交换空间或内存泄漏，也没有 CPU 清理功能。macOS 正常退出应用的请求是独立确认操作，不会回退到强制结束。Windows 性能功能为只读，交换空间为基于提交量的估算值，而非页面文件当前用量。

Docker 管理默认关闭。启用后也仅使用固定命令，排除卷，并单独确认操作不可恢复。

## AI、CLI 与 MCP

安装 Codex 桌面应用并不代表已经安装 Codex CLI。请在应用中检查所用提供商 CLI 的安装、兼容版本和登录状态。本次 Mac 对话流程使用 Codex 验证。虽然也有 Claude Code、Grok、Antigravity、Ollama 适配器，但并未为本次 Mac 发布逐一完成实际运行验证。

应用聊天会向所选提供商发送包含项目名称和大小的有限文件夹摘要、问题与对话历史。本地 CLI 不意味着模型处理离线进行。MCP 清理工具仅提供匿名候选 ID 和有限摘要，不提供批准或执行删除的工具。另行允许文件或文档搜索后，路径和匹配上下文可能传给外部客户端。最终文件操作仍须在应用内确认。

## 平台与开发

需要 Rust stable、Node.js 22+ 和 npm。Windows 还需 WebView2 与 MSVC Build Tools，macOS 还需 Xcode Command Line Tools。

- `apps/desktop/`：主 Tauri/React 应用
- `crates/bloomsweepy-core/`：共享 Rust 分析引擎
- `crates/bloomsweepy-control/`、`apps/bloomsweepy-mcp/`：本地控制协议与 CLI/MCP 桥接
- `BroomSweepy/`：旧版 SwiftUI 参考实现

本次更新已在 Apple Silicon Mac 上构建、安装，并检查本地文件扫描与 Codex 对话流程。最新 Windows 运行验证需单独完成，安装程序由 Windows CI 构建。Mac 验证构建采用 ad-hoc 签名，并非已经 Apple 公证。下载与平台注意事项请查看[发布页](https://github.com/Dannykkh/bloomsweepy/releases)。

```sh
cd apps/desktop
npm ci
npm run tauri dev
```

```sh
# Repository root
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/desktop
npm run check
npm run test:all
npm run build
npm run tauri build
```

## 文档

[更新日志](CHANGELOG.md) · [CLI 连接与控制](docs/cli-control.md) · [性能与内存边界](docs/architecture/startup-memory-status.md) · [安全回收站操作](docs/architecture/safe-trash-actions.md) · [文档搜索](docs/architecture/document-search.md) · [文件搜索](docs/architecture/fast-file-search.md) · [设计](DESIGN.md) · [复现截图](docs/assets/screenshots/README.md)

## 重要提示：数据丢失与恢复责任

BroomSweepy 设计为只处理用户选择并最终确认的项目。但是，是否能够恢复仍可能受到操作系统权限、回收站设置、同步服务，以及外置或网络驱动器状态的影响。Docker 清理不使用操作系统回收站，已完成的步骤无法恢复。

执行清理前，请备份重要数据，并核对所有选定路径、文件和 Docker 分类。除法律规定不得排除的责任外，项目提供者和贡献者不对用户发起的文件移动、删除、清空回收站或 Docker 清理造成的数据丢失或恢复失败承担责任。
