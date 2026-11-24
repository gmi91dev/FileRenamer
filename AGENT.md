# AGENT.md

> 面向 AI 编码助手的项目级指令。在本仓库工作时必须遵守以下约定。

## 项目定位

- **FileRenamer**：批量重命名文件工具。
- 多平台仓库，但**不是跨平台框架**：每个平台目录使用各自**原生技术独立实现**（如 `windows/` 内 C# WinForms 与 Rust/Win32 两套实现互不依赖），不要引入跨平台 UI 框架。
- 公开仓库：密钥、个人路径、内部资料一律不得写入代码、注释与文档。

## 目录结构

```
FileRenamer/
├── AGENT.md        # 本文件（项目级 agent 指令）
├── doc/            # 平台无关文档：总览 doc/README.md（全仓库文档地图）、日记 doc/diary.md
├── windows/        # Windows 平台实现（平台文档 windows/doc/README.md）
│   ├── FileRenamer/        # C# WinForms（net11.0-windows）
│   └── FileRenamerRust/    # Rust + Win32 API
├── mac/            # （预留）
└── linux/          # （预留）
```

## 工作约定

1. **开发前先查官方文档**，禁止凭记忆猜测 API；文档没有的先反馈再等指令。代码注释中注明依据（文档路径 + 关键约束）。
2. **不要擅自新增界面上看得见的元素**（表格列、按钮、卡片、字段等），先询问再动手。
3. **歧义先问**，不要按自己的理解做。
4. **改动方向有歧义时先确认用户真实意图**，再做最小改动。
5. 遵守两级 `.gitignore`：根目录放通用规则，平台目录放平台专属规则；平台产物（`bin/`、`obj/`、`target/`、`publish/`、`.vs/` 等）不得提交。
6. **不提交 `.workbuddy/`**；跨会话交接与知识沉淀一律写入 `doc/`（关键结论，不写流水账）。

## mono-trim 稀疏检出

本仓库用 [mono-trim](https://github.com/gmi91dev/mono-trim) 管理工作区裁剪：

- Windows 端开发执行 `mtrim windev`（black 模式排除 `mac/`、`linux/`）；恢复全量用 `mtrim applyall`。
- 配置文件 `.mono-trim/config.toml` 与 `mono-trim.toml` **随仓库提交**；`.gitignore` 不得误伤它们（若加宽泛忽略规则，需补 `!.mono-trim/`、`!mono-trim.toml` 例外）。
- profile 名即命令名，新增平台 profile 时在 `mono-trim.toml` 中追加并运行 `mtrim doctor` 校验。

## 文档维护

- **`doc/README.md` 是全仓库文档总入口**：所有文档的路径、用途与阅读时机都在其文档地图中登记；Agent 找资料先读它。
- 完成关键工作（架构决策、重要踩坑、结构变更）后，更新对应文档：平台无关 → `doc/diary.md` 追加日期条目；平台相关 → `<平台>/doc/`；新增文档 → 回填 `doc/README.md` 索引。
- 只记录**关键内容**方便回溯，不记录临时信息与敏感信息。
