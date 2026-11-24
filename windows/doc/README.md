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

## 关键实现事实

### 两阶段改名与中转名

批量重命名采用「先全部改为中转名、再全部改为目标名」的两阶段流程，用于规避链式改名冲突与仅大小写差异。

**中转名不得以 `._` 开头**。Samba 系 NAS 会拒绝任何 `._` 前缀的文件名（macOS AppleDouble 双文件前缀），服务端直接返回"文件不存在"，客户端表现为"找不到指定文件"。实测仅 `._` 前缀被拒，`.dot.txt`、`.._x.txt`、`x._y.txt` 均正常。当前中转名统一使用 `__rn_` 前缀。

中转阶段与改名阶段均需捕获异常并回滚：失败时把已改到中转名的文件还原为原名，不残留半成品。

### 清单格式

- 表头固定四列：`no` / `old` / `new` / `author`。
- `no` 列用 `=ROW()-1` 公式生成序号，删行后由 Excel/WPS 重算，序号自动连续。
- `author` 是内置字段：值由用户在表格中填写，可用 `{author}` 参与名称模板；表格没有该列时按空值处理，不算未知变量。
- `old` / `new` 表头名固定不可改；表格其余列的表头可作自定义变量使用。

### 版本号

- 窗口标题为 `FileRenamer v<版本号>`，任务栏与窗口列表可直接识别当前版本。
- C# 读取程序集的 `AssemblyInformationalVersionAttribute`（单文件发布下 `FileVersionInfo` 取不到值）；注意属性名不可叫 `ProductVersion`，会与 WinForms 基类成员冲突。
- Rust 读取 `env!("CARGO_PKG_VERSION")`；窗口标题需运行时拼接，不能用 `w!()` 字面量宏。
- 发布时以 git tag 为唯一版本来源，构建流程把 tag 解析出的版本号注入两套构建配置，确保标题显示、Release 标签、产物文件名三者一致。

