# Windows 平台开发文档

> 平台无关内容见 [根 doc/](../../doc/README.md)。本目录只记录 Windows 平台的关键内容。

## 实现概览

Windows 平台有两套独立实现：

| 项目 | 技术栈 | 位置 |
| --- | --- | --- |
| FileRenamer | C# WinForms（.NET 11） | [FileRenamer/](../FileRenamer/) |
| FileRenamerRust | Rust + Win32 API（原生，非框架封装 UI） | [FileRenamerRust/](../FileRenamerRust/) |

## FileRenamer（C# / WinForms）

关键构建事实（见 `FileRenamer.csproj`）：

- 目标框架 `net11.0-windows`，WinForms，启用 Nullable 与 ImplicitUsings。
- 发布为**单文件自包含**（win-x64），开启 `EnableCompressionInSingleFile`：体积 127MB → 55MB，代价是启动时内存解压。依据：微软官方文档 single-file deployment。
- Excel 能力依赖 **ClosedXML 0.105.1**。

## FileRenamerRust（Rust / Win32）

关键构建事实（见 `Cargo.toml`）：

- edition 2024，通过 `windows` crate 0.62 直接调用 Win32 API（UI、Shell、GDI、文件系统等 feature 集），配合 `app.manifest`（DPI 感知等清单）与 `app.rc`（版本资源），`build.rs` 参与构建。
- Excel 能力：读 **calamine**、写 **rust_xlsxwriter**；另有 md5 校验。
- release 配置**体积优先**：`opt-level = "z"` + LTO + 单 codegen unit + strip + `panic = "abort"`。

## 约定

- 构建产物（`bin/`、`obj/`、`publish/`、`target/`、`.vs/` 等）由 `windows/.gitignore` 统一忽略，勿在项目中残留提交。
