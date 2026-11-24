using System.Reflection;

namespace FileRenamer;

public partial class Form1 : Form
{
    // 旋转加载指示器：耗时操作期间显示，构造后加入窗体（隐藏时不参与 AutoSize 布局）
    private readonly Spinner spinner = new();

    /// <summary>
    /// 产品版本号：取自 csproj 的 Version 属性（SDK 据此生成 AssemblyInformationalVersionAttribute，
    /// 微软文档 https://learn.microsoft.com/dotnet/core/project-sdk/msbuild-props）。
    /// 不用 FileVersionInfo：单文件发布下 Assembly.Location 为空，取不到文件版本资源。
    /// </summary>
    public static string AppVersion =>
        Assembly.GetExecutingAssembly().GetCustomAttribute<AssemblyInformationalVersionAttribute>()
            ?.InformationalVersion
        ?? "unknown";

    public Form1()
    {
        InitializeComponent();
        // 标题栏带版本号：任务栏与窗口列表都能看到当前跑的哪个版本
        Text = $"{Text} v{AppVersion}";
        spinner.Visible = false;
        Controls.Add(spinner);
    }

    /// <summary>
    /// 进入/退出忙状态：显示旋转菊花，禁用浏览/生成/重命名等操作按钮。
    /// 耗时逻辑在后台线程执行（Task.Run），UI 线程保持消息循环，窗口不会无响应。
    /// </summary>
    private void SetBusy(bool busy)
    {
        UseWaitCursor = busy;
        btnBrowse.Enabled = btnBrowseList.Enabled = btnGenerate.Enabled = btnRename.Enabled = !busy;
        if (busy)
        {
            spinner.Location = new Point((ClientSize.Width - spinner.Width) / 2, (ClientSize.Height - spinner.Height) / 2);
            spinner.BringToFront();
            spinner.Visible = true;
        }
        else
        {
            spinner.Visible = false;
        }
    }

    private void btnBrowse_Click(object? sender, EventArgs e)
    {
        using var dialog = new FolderBrowserDialog
        {
            Description = "选择要处理的文件夹",
            ShowNewFolderButton = false,
        };
        if (!string.IsNullOrWhiteSpace(txtFolder.Text) && Directory.Exists(txtFolder.Text))
            dialog.SelectedPath = txtFolder.Text;

        if (dialog.ShowDialog(this) == DialogResult.OK)
            txtFolder.Text = dialog.SelectedPath;
    }

    private void btnBrowseList_Click(object? sender, EventArgs e)
    {
        using var dialog = new OpenFileDialog
        {
            Title = "选择重命名清单文件",
            Filter = "Excel 工作簿 (*.xlsx)|*.xlsx|所有文件 (*.*)|*.*",
            DefaultExt = ".xlsx",
            CheckFileExists = true,
            CheckPathExists = true,
        };
        if (!string.IsNullOrWhiteSpace(txtListFile.Text) && File.Exists(txtListFile.Text))
            dialog.InitialDirectory = Path.GetDirectoryName(txtListFile.Text);

        if (dialog.ShowDialog(this) == DialogResult.OK)
            txtListFile.Text = dialog.FileName;
    }

    private async void btnGenerate_Click(object? sender, EventArgs e)
    {
        string folder = txtFolder.Text.Trim();
        if (string.IsNullOrWhiteSpace(folder) || !Directory.Exists(folder))
        {
            MessageBox.Show(this, "请先选择有效的文件夹。", "提示", MessageBoxButtons.OK, MessageBoxIcon.Information);
            return;
        }

        SetBusy(true);
        try
        {
            // 扫描文件夹 + 写 xlsx 可能耗时，放后台线程，避免阻塞 UI 消息循环
            var names = await Task.Run(() => FileOps.GenerateList(folder, chkVideo.Checked, chkImage.Checked, chkDoc.Checked));
            string listPath = Path.Combine(folder, FileOps.ListFileName);
            MessageBox.Show(this,
                $"已生成清单：{listPath}\n共 {names.Count} 个文件。\n填好 new 列（可选填 author 列）后，可在功能二中随时处理。",
                "完成", MessageBoxButtons.OK, MessageBoxIcon.Information);
        }
        catch (Exception ex)
        {
            MessageBox.Show(this, "生成清单失败：\n" + ex.Message, "错误", MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
        finally
        {
            SetBusy(false);
        }
    }

    private async void btnRename_Click(object? sender, EventArgs e)
    {
        string listPath = txtListFile.Text.Trim();
        if (string.IsNullOrWhiteSpace(listPath) || !File.Exists(listPath))
        {
            MessageBox.Show(this, "请先选择有效的清单文件。", "提示", MessageBoxButtons.OK, MessageBoxIcon.Information);
            return;
        }

        string template = txtPrefix.Text.Trim();
        if (template.Length == 0)
        {
            MessageBox.Show(this, "请填写名称格式，例如 {new}.{ext}", "提示", MessageBoxButtons.OK, MessageBoxIcon.Information);
            return;
        }

        SetBusy(true);
        try
        {
            // 读取清单 + MD5 哈希全部文件可能耗时，放后台线程，避免阻塞 UI 消息循环
            var result = await Task.Run(() => FileOps.Rename(listPath, template));
            string unknownHint = result.Unknown.Count == 0
                ? ""
                : "\n\n未定义变量（已按空值处理）：" + string.Join("、", result.Unknown);

            if (!result.Valid)
            {
                MessageBox.Show(this, "发现以下问题，未执行任何重命名：\n\n" + Join(result.Problems) +
                    (result.Problems.Count > 20 ? $"\n……等共 {result.Problems.Count} 项" : "") + unknownHint,
                    "无法继续", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            }
            else if (result.Failures.Count == 0)
            {
                MessageBox.Show(this, $"重命名完成，共处理 {result.Done} 个文件。" + unknownHint,
                    "完成", MessageBoxButtons.OK, MessageBoxIcon.Information);
            }
            else
            {
                MessageBox.Show(this, $"完成 {result.Done} 个，失败 {result.Failures.Count} 个：\n\n" + Join(result.Failures) + unknownHint,
                    "部分失败", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            }
        }
        catch (Exception ex)
        {
            MessageBox.Show(this, "重命名失败：\n" + ex.Message, "错误", MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
        finally
        {
            SetBusy(false);
        }
    }

    private void btnVarsHelp_Click(object? sender, EventArgs e)
    {
        string help = string.Join("\n", new[]
        {
            $"FileRenamer v{AppVersion}",
            "",
            "【内置变量】（表格第一行的列名也可直接当作 {变量} 使用）",
            "",
            "no       序号，按实际处理行从 1 编号；空行不占号，忽略表格 no 列的值",
            "old      旧文件名主名，不含后缀",
            "new      表格 new 列填写的新名称",
            "author   表格 author 列填写的作者名（生成清单时 new 右侧的固定列）",
            "ext      文件后缀，小写、不带点，如 mp4",
            "create   文件创建日期，格式 YYYY-MM-DD",
            "update   文件最后修改日期，格式 YYYY-MM-DD",
            "size     文件大小，人类可读，如 1.2 MB",
            "year     修改日期的年份，如 2026",
            "month    修改日期的月份，两位 01-12",
            "day      修改日期的日，两位 01-31",
            "folder   清单文件所在文件夹的名称",
            "hash     文件内容的 MD5，完整 32 位小写",
            "now      执行重命名当天的日期，格式 YYYY-MM-DD",
            "",
            "【注意事项】",
            "1. old 和 new 两列的表头名固定，不能改名，否则无法定位文件。",
            "2. 内置变量优先于表格同名列：即使表格里有 create/size 列，也会被文件实际信息覆盖。",
            "3. author 是固定列名（生成清单时 new 右侧自带），值由你在表里填写，清单无此列时按空值处理。",
            "4. 表格右侧可加自定义列，表头写什么就能用 {什么}，例如列名“作者”→ {作者}。",
            "5. 模板中写了不存在且非内置的变量（如 {date}），会替换为空并在完成时列出提醒。",
            "6. {no} 按实际处理的行重新编号、跳过空行，因此可能与原表格行号不同。",
            "7. 新名称不能含 \\ / : * ? \" < > | 等字符；后缀统一输出小写。",
        });
        MessageBox.Show(this, help, "变量说明", MessageBoxButtons.OK, MessageBoxIcon.Information);
    }

    private static string Join(IEnumerable<string> items) => string.Join("\n", items.Take(20));
}
