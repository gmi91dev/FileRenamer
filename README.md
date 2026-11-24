# FileRenamer

批量重命名文件：从文件夹生成 Excel 清单 → 在表格中填好新名称 → 一键按清单重命名。

> 多平台仓库：每个平台使用各自**原生技术**独立实现，不依赖跨平台框架。当前提供 Windows 版（C# 与 Rust 两套实现），macOS / Linux 预留。

## 功能特性

- **清单式重命名**：扫描目标文件夹，生成 `重命名清单.xlsx`（`no` / `old` / `new` 三列），在 Excel 中编辑新名称后按清单批量执行，改前可审、可改、可留档。
- **类型过滤**：按视频 / 图片 / 文档三类扩展名筛选参与重命名的文件，全不选则处理全部文件。
- **与资源管理器一致的排序**：清单按名称升序生成，采用 `StrCmpLogicalW`（数字按数值比较），顺序与 Windows 资源管理器显示一致。
- **长文件名友好**：清单 `old` / `new` 列自动换行，不遮挡。

## Windows 版

| 实现 | 技术栈 | 说明 |
| --- | --- | --- |
| [windows/FileRenamer](windows/FileRenamer/) | C# WinForms（.NET 11） | 主实现，单文件自包含发布（win-x64，压缩后约 55MB） |
| [windows/FileRenamerRust](windows/FileRenamerRust/) | Rust + Win32 API | 原生实现（不依赖 UI 框架），PerMonitorV2 DPI 感知，release 体积优先（LTO + strip） |

两套实现的业务逻辑行为保持一致。

## 构建与发布

**C# 版**（需要 .NET SDK）：

```powershell
cd windows/FileRenamer
dotnet publish -c Release   # 产物在 bin/Release/net11.0-windows/win-x64/publish/
```

**Rust 版**（需要 Rust 工具链）：

```powershell
cd windows/FileRenamerRust
cargo build --release   # 产物在 target/release/
```

## 项目结构

```
FileRenamer/
├── AGENT.md        # 项目级 AI 助手工作约定
├── doc/            # 平台无关文档（总览、开发日记）
└── windows/        # Windows 平台实现（含平台文档 windows/doc/）
```

更多文档入口见 [doc/README.md](doc/README.md)。

## 许可证

[MIT](LICENSE) © 2026 gmi91
