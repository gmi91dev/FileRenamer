using System.Runtime.InteropServices;
using System.Security.Cryptography;
using ClosedXML.Excel;

namespace FileRenamer;

/// <summary>
/// 文件处理核心逻辑：生成清单、按清单重命名。
/// 与界面解耦，便于独立测试。
/// </summary>
public static class FileOps
{
    public const string ListFileName = "重命名清单.xlsx";

    // 与 Windows 资源管理器一致的文件名升序比较：StrCmpLogicalW（shlwapi.dll）。
    // 微软文档：数字按数值比较、不区分大小写；CharSet.Unicode 将 string 封送为 Unicode，
    // ExactSpelling=true 精确匹配导出名，不做 A/W 后缀探测。
    // https://learn.microsoft.com/en-us/windows/win32/api/shlwapi/nf-shlwapi-strcmplogicalw
    [DllImport("shlwapi.dll", CharSet = CharSet.Unicode, ExactSpelling = true)]
    private static extern int StrCmpLogicalW(string psz1, string psz2);

    // 类型扩展名映射（全不选表示所有文件）
    private static readonly Dictionary<string, string[]> TypeExtensions = new()
    {
        ["video"] = new[] { ".mp4", ".avi", ".mkv", ".mov", ".wmv", ".flv", ".webm", ".m4v", ".mpg", ".mpeg", ".3gp", ".ts", ".rmvb", ".rm", ".vob" },
        ["image"] = new[] { ".jpg", ".jpeg", ".png", ".gif", ".bmp", ".webp", ".tif", ".tiff", ".ico", ".heic", ".svg" },
        ["doc"]   = new[] { ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx", ".pdf", ".txt", ".rtf", ".md", ".csv", ".odt", ".ods", ".odp" },
    };

    /// <summary>生成清单 xlsx，返回写入的文件名列表（已按名称升序）。</summary>
    public static List<string> GenerateList(string folder, bool video, bool image, bool doc)
    {
        var selectedTypes = new List<string>();
        if (video) selectedTypes.Add("video");
        if (image) selectedTypes.Add("image");
        if (doc) selectedTypes.Add("doc");

        var allowed = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var type in selectedTypes)
            foreach (var ext in TypeExtensions[type]) allowed.Add(ext);

        var names = Directory.GetFiles(folder)
            .Select(Path.GetFileName)
            .Where(name => !string.IsNullOrEmpty(name))
            .Where(name => !string.Equals(name, ListFileName, StringComparison.OrdinalIgnoreCase))
            .Where(name => selectedTypes.Count == 0 || allowed.Contains(Path.GetExtension(name!)))
            .Cast<string>()
            .ToList();

        // 名称升序：与 Windows 资源管理器一致（StrCmpLogicalW：数字按数值、符号/中文按系统权重）
        names.Sort(static (a, b) => StrCmpLogicalW(a, b));

        string listPath = Path.Combine(folder, ListFileName);
        // 表头即变量名：no=序号，old=完整旧文件名（含后缀，程序据此定位文件），new=新名称
        using var workbook = new XLWorkbook();
        var ws = workbook.AddWorksheet("重命名清单");
        ws.Cell(1, 1).Value = "no";
        ws.Cell(1, 2).Value = "old";
        ws.Cell(1, 3).Value = "new";
        // 标题行：加粗 + 水平居中（ClosedXML 官方 wiki: Style.Alignment.Horizontal = XLAlignmentHorizontalValues.Center）
        for (int c = 1; c <= 3; c++)
        {
            ws.Cell(1, c).Style.Font.Bold = true;
            ws.Cell(1, c).Style.Alignment.Horizontal = XLAlignmentHorizontalValues.Center;
        }

        for (int i = 0; i < names.Count; i++)
        {
            ws.Cell(i + 2, 1).Value = i + 1;
            // 序号列数据：水平居中
            ws.Cell(i + 2, 1).Style.Alignment.Horizontal = XLAlignmentHorizontalValues.Center;
            ws.Cell(i + 2, 2).Value = names[i];
        }

        // old/new 列开启自动换行：长文件名不会被相邻列遮挡（IXLAlignment.WrapText）
        ws.Column(2).Style.Alignment.WrapText = true;
        ws.Column(3).Style.Alignment.WrapText = true;

        ws.Column(1).Width = 6;
        ws.Column(2).Width = 44;
        ws.Column(3).Width = 32;
        workbook.SaveAs(listPath);
        return names;
    }

    public sealed class RenameResult
    {
        public int Done { get; set; }
        public List<string> Problems { get; } = new();   // 校验阶段发现的问题（未执行任何改名）
        public List<string> Failures { get; } = new();   // 执行阶段失败
        public List<string> Unknown { get; } = new();    // 模板里引用但未定义的变量
        public bool Valid => Problems.Count == 0;
    }

    private sealed record RenamePlan(string OldName, string NewName);

    /// <summary>把模板中的 {变量} 替换为对应值；未定义变量替换为空并收集名字。支持 {{ / }} 转义。</summary>
    private static string Render(string template, Dictionary<string, string> values, HashSet<string> unknown)
    {
        var sb = new System.Text.StringBuilder();
        for (int i = 0; i < template.Length;)
        {
            if (template[i] == '{' && i + 1 < template.Length && template[i + 1] == '{')
            { sb.Append('{'); i += 2; continue; }
            if (template[i] == '}' && i + 1 < template.Length && template[i + 1] == '}')
            { sb.Append('}'); i += 2; continue; }
            if (template[i] == '{')
            {
                int end = template.IndexOf('}', i + 1);
                if (end > i)
                {
                    string key = template.Substring(i + 1, end - i - 1);
                    if (values.TryGetValue(key, out var value)) sb.Append(value);
                    else unknown.Add(key);
                    i = end + 1;
                    continue;
                }
            }
            sb.Append(template[i]);
            i++;
        }
        return sb.ToString();
    }

    private static string HumanSize(long bytes)
    {
        const double KB = 1024.0;
        if (bytes < 1024) return $"{bytes} B";
        if (bytes < 1024 * 1024) return $"{bytes / KB:F1} KB";
        if (bytes < 1024L * 1024 * 1024) return $"{bytes / (KB * KB):F1} MB";
        return $"{bytes / (KB * KB * KB):F1} GB";
    }

    /// <summary>读取指定清单 xlsx 并执行重命名；处理范围 = 清单文件所在文件夹。与功能一相互独立。</summary>
    public static RenameResult Rename(string listPath, string template)
    {
        template ??= string.Empty;
        var result = new RenameResult();
        string folder = Path.GetDirectoryName(Path.GetFullPath(listPath)) ?? ".";
        string listFileName = Path.GetFileName(listPath);

        var plans = new List<RenamePlan>();
        using (var workbook = new XLWorkbook(listPath))
        {
            var ws = workbook.Worksheet(1);
            int lastRow = ws.LastRowUsed()?.RowNumber() ?? 0;

            // 第一行表头即变量名
            var headers = new List<string>();
            int lastCol = ws.LastColumnUsed()?.ColumnNumber() ?? 1;
            for (int c = 1; c <= lastCol; c++)
                headers.Add(ws.Cell(1, c).GetString().Trim());

            int oldCol = headers.IndexOf("old");
            int newCol = headers.IndexOf("new");
            if (oldCol < 0 || newCol < 0)
                throw new InvalidDataException("清单缺少表头：需包含 old（完整旧文件名）和 new（新名称）两列。");

            // 模板不用 {new} 时（如 {no}_{old}.{ext}），new 列留空也应正常处理
            bool usesNew = template.Contains("{new}", StringComparison.Ordinal);
            string folderName = new DirectoryInfo(folder).Name;
            var now = DateTime.Now;
            var unknown = new HashSet<string>(StringComparer.Ordinal);
            int seq = 0; // {no} 为内置变量：按实际处理行从 1 重新编号，不读表格 no 列

            for (int row = 2; row <= lastRow; row++)
            {
                string Get(int colIndex) => colIndex >= 0 && colIndex < headers.Count
                    ? ws.Cell(row, colIndex + 1).GetString().Trim() : string.Empty;

                string oldFull = Get(oldCol);
                string newBase = Get(newCol);
                if (oldFull.Length == 0 || (usesNew && newBase.Length == 0)) continue;
                seq++;

                string source = Path.Combine(folder, oldFull);
                string ext = Path.GetExtension(oldFull).TrimStart('.').ToLowerInvariant();
                string stem = Path.GetFileNameWithoutExtension(oldFull);

                // 表格列值：表头即变量名
                var values = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
                for (int c = 0; c < headers.Count; c++)
                    if (headers[c].Length > 0) values[headers[c]] = Get(c);

                // 内置变量（覆盖表格同名列）
                values["no"] = seq.ToString(); // 按处理顺序重新编号，忽略表格 no 列的值
                values["old"] = stem;
                values["ext"] = ext;
                values["folder"] = folderName;
                values["hash"] = Convert.ToHexString(MD5.HashData(File.ReadAllBytes(source))).ToLowerInvariant();
                try
                {
                    var info = new FileInfo(source);
                    values["size"] = HumanSize(info.Length);
                    values["create"] = info.CreationTime.ToString("yyyy-MM-dd");
                    values["update"] = info.LastWriteTime.ToString("yyyy-MM-dd");
                    values["year"] = info.LastWriteTime.Year.ToString();
                    values["month"] = info.LastWriteTime.Month.ToString("D2");
                    values["day"] = info.LastWriteTime.Day.ToString("D2");
                }
                catch { /* 文件时间读取失败时该变量留空 */ }
                values["now"] = now.ToString("yyyy-MM-dd");

                plans.Add(new RenamePlan(oldFull, Render(template, values, unknown)));
            }
            foreach (var u in unknown) result.Unknown.Add(u);
        }

        if (plans.Count == 0) return result;

        // 校验
        var seenNew = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var oldNames = new HashSet<string>(plans.Select(p => p.OldName), StringComparer.OrdinalIgnoreCase);
        char[] invalid = Path.GetInvalidFileNameChars();

        foreach (var plan in plans)
        {
            if (string.IsNullOrWhiteSpace(plan.NewName))
            {
                result.Problems.Add($"模板生成了空文件名：{plan.OldName}");
                continue;
            }
            if (!File.Exists(Path.Combine(folder, plan.OldName)))
            {
                result.Problems.Add($"源文件不存在：{plan.OldName}");
                continue;
            }
            if (plan.NewName.IndexOfAny(invalid) >= 0)
            {
                result.Problems.Add($"新名称含非法字符：{plan.NewName}");
                continue;
            }
            if (string.Equals(plan.NewName, listFileName, StringComparison.OrdinalIgnoreCase))
            {
                result.Problems.Add($"新名称不能与清单同名：{plan.NewName}");
                continue;
            }
            if (!seenNew.Add(plan.NewName))
            {
                result.Problems.Add($"新名称重复：{plan.NewName}");
                continue;
            }
            string newPath = Path.Combine(folder, plan.NewName);
            if (File.Exists(newPath) && !oldNames.Contains(plan.NewName))
                result.Problems.Add($"目标文件已存在：{plan.NewName}");
        }

        if (!result.Valid) return result;

        // 两阶段改名：先全部改临时名，再改目标名，规避链式冲突与仅大小写差异
        var staged = new List<(string TempPath, RenamePlan Plan)>();
        foreach (var plan in plans)
        {
            string tempPath = Path.Combine(folder, $".__rn_{Guid.NewGuid():N}.tmp");
            File.Move(Path.Combine(folder, plan.OldName), tempPath);
            staged.Add((tempPath, plan));
        }

        foreach (var (tempPath, plan) in staged)
        {
            try
            {
                File.Move(tempPath, Path.Combine(folder, plan.NewName));
                result.Done++;
            }
            catch (Exception ex)
            {
                result.Failures.Add($"{plan.OldName} → {plan.NewName}：{ex.Message}");
            }
        }
        return result;
    }
}
