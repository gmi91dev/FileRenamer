# 开发日记（平台无关）

> 只记关键事件：重要决策、结构变更、踩坑结论。细节见对应平台文档。

## 2026-11-24

- 仓库迁入并初始化工程结构：确立多平台原生开发模式（`windows/` 起步，`mac/`、`linux/` 预留）。
- 建立两级 `.gitignore`：根目录通用规则 + 平台目录专属规则（Windows 侧覆盖 .NET 与 Rust 构建产物）。
- 接入 [mono-trim](https://github.com/gmi91dev/mono-trim) 做稀疏检出管理，配置 `windev` profile（black 模式排除 `mac/`、`linux/`），`mtrim doctor` 校验通过并实测裁剪成功。
- 建立 `doc/` 文档目录与平台文档规范。
- 根目录新增 `AGENT.md`：固化项目级工作约定（原生多平台定位、先查文档不猜 API、不擅自加界面元素、歧义先问、.workbuddy 不提交、mono-trim 约定），供 AI 编码助手遵循。
- 将 `doc/README.md` 升级为全仓库文档总览：收录根目录全部文件、mono-trim 两份配置、各平台文档的路径/用途/阅读时机，并规定"新增文档必须回填索引"，作为 agent 与开发者的统一查档入口。
- 填充根 `README.md`（仅对外介绍，不做文档统筹）：基于源码实读撰写功能特性（Excel 清单式重命名工作流、类型过滤、StrCmpLogicalW 排序、两套 Windows 实现）、构建命令与项目结构。
- 接入 GitHub Actions 自动发布：新增 `.github/workflows/release.yml`，push `v*` 标签即在 windows-latest 上构建 C#（dotnet publish 单文件自包含）与 Rust（cargo release）两个 exe，用官方 `gh release create` 创建 Release 并以 `FileRenamer-v<版本>-csharp/rust-win-x64.exe` 命名上传；刻意不引入第三方 action（除官方 checkout/setup-dotnet）。
- 发布首个版本 v0.1.0（C# + Rust 两个 exe），验证 workflow 全链路跑通。
- 修复 UI 无响应：两套实现（C# WinForms / Rust Win32）的生成清单与重命名改为后台线程执行，窗口保持响应；执行期间显示自绘旋转菊花并禁用 浏览×2/生成/重命名 四个操作按钮（C# 用 Task.Run + 自绘 Spinner 控件；Rust 用 std::thread + PostMessageW 回 UI 线程 + GDI Arc 菊花子窗口）。
- 性能优化：名称格式模板未使用 {hash} 时跳过 MD5 全文件读取（此前每个文件都强制计算，大文件场景代价高）。
- 发布流程增强：Release 创建改用 `gh release create --generate-notes`，每次发布自动生成基于提交记录的发布内容。发布 v0.1.1。
