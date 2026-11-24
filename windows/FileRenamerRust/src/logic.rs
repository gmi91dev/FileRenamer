//! 核心业务逻辑：生成清单 xlsx、按清单重命名。
//! 与 WinForms 版 FileOps.cs 行为保持一致。

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use calamine::{Data, Reader, Xlsx, open_workbook};
use rust_xlsxwriter::{Format, FormatAlign, Workbook};
use windows::core::PCWSTR;
use windows::Win32::UI::Shell::StrCmpLogicalW;

use crate::vars;

/// 与 Windows 资源管理器一致的文件名升序排序：StrCmpLogicalW（shlwapi.dll）。
/// 依据微软官方文档：数字按数值比较、不区分大小写，符号/中文按系统排序权重。
/// https://learn.microsoft.com/en-us/windows/win32/api/shlwapi/nf-shlwapi-strcmplogicalw
/// 注意：文档说明该函数的排序行为可能随 Windows 版本变化，故用于"与本机资源管理器一致"而非规范化排序。
fn sort_names(names: &mut Vec<String>) {
    // 预生成 UTF-16（尾部补 NUL），避免每次比较都重新编码
    let mut wide: Vec<(Vec<u16>, String)> = names
        .drain(..)
        .map(|s| {
            let mut w: Vec<u16> = s.encode_utf16().collect();
            w.push(0);
            (w, s)
        })
        .collect();
    wide.sort_by(|a, b| {
        let r = unsafe {
            StrCmpLogicalW(
                PCWSTR::from_raw(a.0.as_ptr()),
                PCWSTR::from_raw(b.0.as_ptr()),
            )
        };
        r.cmp(&0)
    });
    *names = wide.into_iter().map(|(_, s)| s).collect();
}

pub const LIST_FILE_NAME: &str = "重命名清单.xlsx";

const VIDEO: &[&str] = &[".mp4", ".avi", ".mkv", ".mov", ".wmv", ".flv", ".webm", ".m4v", ".mpg", ".mpeg", ".3gp", ".ts", ".rmvb", ".rm", ".vob"];
const IMAGE: &[&str] = &[".jpg", ".jpeg", ".png", ".gif", ".bmp", ".webp", ".tif", ".tiff", ".ico", ".heic", ".svg"];
const DOC: &[&str] = &[".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx", ".pdf", ".txt", ".rtf", ".md", ".csv", ".odt", ".ods", ".odp"];

pub struct TypeFilter {
    pub video: bool,
    pub image: bool,
    pub doc: bool,
}

/// 生成清单 xlsx，返回写入的文件名列表（已按名称升序）。
pub fn generate_list(folder: &Path, filter: &TypeFilter) -> Result<Vec<String>, Box<dyn Error>> {
    let mut allowed: HashSet<String> = HashSet::new();
    if filter.video { for e in VIDEO { allowed.insert(e.to_string()); } }
    if filter.image { for e in IMAGE { allowed.insert(e.to_string()); } }
    if filter.doc { for e in DOC { allowed.insert(e.to_string()); } }
    let any_selected = filter.video || filter.image || filter.doc;

    let mut names: Vec<String> = Vec::new();
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() { continue; }
        let name: OsString = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.eq_ignore_ascii_case(LIST_FILE_NAME) { continue; }

        if any_selected {
            let ext = Path::new(name).extension().and_then(|x| x.to_str()).unwrap_or("");
            let ext_lower = format!(".{}", ext.to_lowercase());
            if !allowed.contains(&ext_lower) { continue; }
        }
        names.push(name.to_string());
    }

    // 名称升序：与 Windows 资源管理器一致（StrCmpLogicalW，见 sort_names 文档）
    sort_names(&mut names);

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("重命名清单")?; // 与 rename() 读取时的工作表名保持一致
    // 表头即变量名：no=序号，old=完整旧文件名（含后缀），new=新名称（用户填）。
    // old 列必须存完整文件名，否则程序无法定位原文件；{old} 变量渲染时自动剥离后缀。
    // 标题行：加粗 + 水平居中；序号列数据：水平居中；全部内容垂直居中。
    // 依据 rust_xlsxwriter 官方示例 Format::set_align(FormatAlign::Center)，
    // 垂直居中同为 set_align(FormatAlign::VerticalCenter)。
    let bold_center = Format::new().set_bold()
        .set_align(FormatAlign::Center).set_align(FormatAlign::VerticalCenter);
    let center = Format::new()
        .set_align(FormatAlign::Center).set_align(FormatAlign::VerticalCenter);
    // 依据 rust_xlsxwriter 文档 Format::set_text_wrap()：文本超列宽时自动换行，
    // 避免长文件名被右侧相邻列截断遮挡。列格式 + 单元格格式同时设置。
    let wrap = Format::new().set_text_wrap().set_align(FormatAlign::VerticalCenter);
    worksheet.write_with_format(0, 0, "no", &bold_center)?;
    worksheet.write_with_format(0, 1, "old", &bold_center)?;
    worksheet.write_with_format(0, 2, "new", &bold_center)?;
    for (i, name) in names.iter().enumerate() {
        // no 列用公式生成序号：用户删掉任意一行后，Excel/WPS 重算公式，序号自动连续不断档。
        // ROW() 返回当前行号，减去表头占用的 1 行即为从 1 开始的序号。
        // 依据 rust_xlsxwriter 文档 write_formula_with_format(row, col, formula, format)。
        worksheet.write_formula_with_format((i + 1) as u32, 0, "=ROW()-1", &center)?;
        worksheet.write_with_format((i + 1) as u32, 1, name.as_str(), &wrap)?;
    }
    worksheet.set_column_width(0, 6)?;
    worksheet.set_column_width(1, 44)?;
    worksheet.set_column_width(2, 32)?;
    worksheet.set_column_format(1, &wrap)?;
    worksheet.set_column_format(2, &wrap)?;
    workbook.save(folder.join(LIST_FILE_NAME))?;
    Ok(names)
}

pub struct RenameResult {
    pub done: usize,
    pub problems: Vec<String>,
    pub failures: Vec<String>,
    pub unknown: Vec<String>, // 模板里引用但清单表头/内置变量都不存在的名字
}

struct Plan { old: String, new: String }

/// 读取指定清单 xlsx 并执行重命名；处理范围 = 清单文件所在文件夹。
/// 表头即变量名：表格列值 + 文件系统内置变量一起填充模板 {变量}。
/// 与功能一（生成清单）相互独立。
pub fn rename(list_path: &Path, template: &str) -> Result<RenameResult, Box<dyn Error>> {
    let mut result = RenameResult {
        done: 0, problems: Vec::new(), failures: Vec::new(), unknown: Vec::new(),
    };
    let folder = list_path.parent().ok_or("清单文件路径无效")?;
    let folder = if folder.as_os_str().is_empty() { Path::new(".") } else { folder };
    let list_file_name = list_path.file_name()
        .and_then(|n| n.to_str())
        .ok_or("清单文件名无效")?;

    let mut workbook: Xlsx<_> = open_workbook(list_path)?;
    let range = workbook.worksheet_range("重命名清单")?;
    let mut rows = range.rows();

    // 第一行表头即变量名
    let header = rows.next().ok_or("清单为空：缺少表头行")?;
    let headers: Vec<String> = header.iter()
        .map(|c| cell_text(Some(c)).trim().to_string())
        .collect();
    let old_col = headers.iter().position(|h| h == "old")
        .ok_or("清单缺少 old 列（旧文件名列的表头须为 old）")?;
    let new_col = headers.iter().position(|h| h == "new")
        .ok_or("清单缺少 new 列（新名称列的表头须为 new）")?;

    // 模板不用 {new} 时（如 {no}_{old}.{ext}），new 列留空也应正常处理
    let template_uses_new = template.contains("{new}");
    // {hash} 需要读全文件做 MD5，代价大：模板未用到时跳过，避免无谓的整文件读取
    let template_uses_hash = template.contains("{hash}");
    let today = unsafe { vars::today() };
    let folder_name = folder.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut plans: Vec<Plan> = Vec::new();
    let mut seq: usize = 0; // {no} 为内置变量：按实际处理的行从 1 重新编号，不读表格 no 列
    for row in rows {
        let cell = |idx: usize| -> String { cell_text(row.get(idx)).trim().to_string() };
        let old_full = cell(old_col);
        let new_base = cell(new_col);
        if old_full.is_empty() || (template_uses_new && new_base.is_empty()) { continue; }
        seq += 1;

        let source = folder.join(&old_full);
        let ext = Path::new(&old_full).extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_lowercase();
        let stem = Path::new(&old_full).file_stem()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_string();

        // 表格列值：表头即变量名
        let mut values: HashMap<String, String> = HashMap::new();
        for (i, name) in headers.iter().enumerate() {
            if !name.is_empty() { values.insert(name.clone(), cell(i)); }
        }

        // 内置变量（覆盖表格同名列）
        values.insert("no".into(), seq.to_string()); // 按处理顺序重新编号，忽略表格 no 列的值
        values.insert("old".into(), stem);          // 旧名主名，不含后缀
        values.insert("ext".into(), ext);           // 小写后缀，不含点
        values.insert("folder".into(), folder_name.clone());
        if template_uses_hash {
            values.insert("hash".into(), vars::file_md5(&source)); // 完整 MD5，小写 32 位（仅模板用到时才读文件计算）
        }
        if let Ok(md) = fs::metadata(&source) {
            values.insert("size".into(), vars::human_size(md.len()));
        }
        let (created, modified) = unsafe { vars::file_dates(&source) };
        if let Some((y, m, d)) = created {
            values.insert("create".into(), vars::format_date(y, m, d));
        }
        if let Some((y, m, d)) = modified {
            values.insert("update".into(), vars::format_date(y, m, d));
            values.insert("year".into(), y.to_string());
            values.insert("month".into(), format!("{:02}", m));
            values.insert("day".into(), format!("{:02}", d));
        }
        if let Some((y, m, d)) = today {
            values.insert("now".into(), vars::format_date(y, m, d));
        }

        let rendered = vars::render(template, &values);
        for unknown in rendered.unknown {
            if !result.unknown.contains(&unknown) { result.unknown.push(unknown); }
        }
        plans.push(Plan { old: old_full, new: rendered.name });
    }

    // 校验
    let mut seen_new: HashSet<String> = HashSet::new();
    let old_names: HashSet<String> = plans.iter().map(|p| p.old.to_lowercase()).collect();
    for p in &plans {
        if p.new.trim().is_empty() {
            result.problems.push(format!("模板生成了空文件名：{}", p.old));
            continue;
        }
        if !folder.join(&p.old).is_file() {
            result.problems.push(format!("源文件不存在：{}", p.old));
            continue;
        }
        if p.new.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']) {
            result.problems.push(format!("新名称含非法字符：{}", p.new));
            continue;
        }
        if p.new.eq_ignore_ascii_case(list_file_name) {
            result.problems.push(format!("新名称不能与清单同名：{}", p.new));
            continue;
        }
        if !seen_new.insert(p.new.to_lowercase()) {
            result.problems.push(format!("新名称重复：{}", p.new));
            continue;
        }
        let target = folder.join(&p.new);
        if target.exists() && !old_names.contains(&p.new.to_lowercase()) {
            result.problems.push(format!("目标文件已存在：{}", p.new));
        }
    }
    if !result.problems.is_empty() { return Ok(result); }

    // 两阶段改名：先全部改临时名，再改目标名，规避链式冲突与仅大小写差异
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let mut staged: Vec<(PathBuf, Plan)> = Vec::new();
    for (i, p) in plans.into_iter().enumerate() {
        let tmp = folder.join(format!(".__rn_{}_{}.tmp", nanos, i));
        fs::rename(folder.join(&p.old), &tmp)?;
        staged.push((tmp, p));
    }
    for (tmp, p) in staged {
        match fs::rename(&tmp, folder.join(&p.new)) {
            Ok(()) => result.done += 1,
            Err(e) => result.failures.push(format!("{} → {}：{}", p.old, p.new, e)),
        }
    }
    Ok(result)
}

fn cell_text(cell: Option<&Data>) -> String {
    match cell {
        Some(Data::String(s)) => s.clone(),
        Some(Data::Float(f)) => {
            if f.fract() == 0.0 { format!("{}", *f as i64) } else { format!("{}", f) }
        }
        Some(Data::Int(i)) => i.to_string(),
        Some(Data::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TMP_SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    fn tmpdir() -> PathBuf {
        let seq = TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let d = std::env::temp_dir().join(format!("rust_renamer_test_{}_{}", std::process::id(), seq));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn write_sheet(path: &Path, headers: &[&str], rows: &[Vec<String>]) {
        let mut w = Workbook::new();
        let ws = w.add_worksheet();
        ws.set_name("重命名清单").unwrap();
        for (c, h) in headers.iter().enumerate() {
            ws.write(0, c as u16, *h).unwrap();
        }
        for (r, row) in rows.iter().enumerate() {
            for (c, v) in row.iter().enumerate() {
                ws.write((r + 1) as u32, c as u16, v.as_str()).unwrap();
            }
        }
        w.save(path).unwrap();
    }

    fn names_in(dir: &Path) -> Vec<String> {
        fs::read_dir(dir).unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect()
    }

    /// 排序须与 Windows 资源管理器一致：符号开头在最前、数字按数值、字母在中文前。
    /// 用相对位置断言而非硬编码全序列，避免依赖具体符号之间的内部顺序（可能随系统版本变化）。
    #[test]
    fn explorer_like_sort_order() {
        let mut names: Vec<String> = [
            "10.mp4", "2.mp4", "g.txt", "l.txt", "⚫️标记.jpg", "【文档.pdf", "abc.mkv", "中文.docx",
        ].iter().map(|s| s.to_string()).collect();
        sort_names(&mut names);
        let pos = |s: &str| names.iter().position(|n| n == s).unwrap();
        assert!(
            ["【文档.pdf", "⚫️标记.jpg"].contains(&names[0].as_str()),
            "符号开头的文件名应排在最前：{:?}", names
        );
        assert!(pos("⚫️标记.jpg") < pos("2.mp4"), "符号应在数字前：{:?}", names);
        assert!(pos("2.mp4") < pos("10.mp4"), "数字按数值比较：{:?}", names);
        assert!(pos("10.mp4") < pos("abc.mkv"), "数字在字母前：{:?}", names);
        assert!(pos("abc.mkv") < pos("中文.docx"), "字母在中文前：{:?}", names);
        assert_eq!(names.last().unwrap(), "中文.docx", "中文应排在最后：{:?}", names);
    }

    #[test]
    fn full_flow() {
        let d = tmpdir();
        let seed = ["旅游 记录.MP4", "风景.JPG", "报告.DOCX", "notes.TXT", "a.mkv", "b.png", "z.pdf", "电影.AVI"];
        for f in seed { fs::write(d.join(f), b"x").unwrap(); }

        // 全类型 + 升序（与资源管理器一致的排序入口）
        let all = generate_list(&d, &TypeFilter { video: false, image: false, doc: false }).unwrap();
        assert_eq!(all.len(), seed.len());
        let mut expect = all.clone();
        sort_names(&mut expect);
        assert_eq!(all, expect, "名称升序");
        assert!(!all.iter().any(|n| n == LIST_FILE_NAME), "清单自身不计入");

        // 检查生成的表头
        let mut wb: Xlsx<_> = open_workbook(d.join(LIST_FILE_NAME)).unwrap();
        let range = wb.worksheet_range("重命名清单").unwrap();
        let first: Vec<String> = range.rows().next().unwrap().iter()
            .map(|c| cell_text(Some(c))).collect();
        assert_eq!(&first[..3], &["no".to_string(), "old".to_string(), "new".to_string()]);
        let rows: Vec<Vec<Data>> = range.rows().map(|r| r.to_vec()).collect();
        drop(wb);

        // 仅视频
        let vids = generate_list(&d, &TypeFilter { video: true, image: false, doc: false }).unwrap();
        assert_eq!(vids.len(), 3, "仅视频应为3个, 实际 {:?}", vids);

        // 重新生成全量清单，在 new 列填新名（保留 no/old 两列）
        generate_list(&d, &TypeFilter { video: false, image: false, doc: false }).unwrap();
        let mut wb: Xlsx<_> = open_workbook(d.join(LIST_FILE_NAME)).unwrap();
        let range = wb.worksheet_range("重命名清单").unwrap();
        let rows: Vec<Vec<Data>> = range.rows().map(|r| r.to_vec()).collect();
        drop(wb);
        let mut w = Workbook::new();
        let ws = w.add_worksheet();
        ws.set_name("重命名清单").unwrap();
        for (c, h) in ["no", "old", "new"].iter().enumerate() { ws.write(0, c as u16, *h).unwrap(); }
        for (ri, row) in rows.iter().enumerate().skip(1) {
            let no = cell_text(row.get(0));
            let old = cell_text(row.get(1));
            ws.write(ri as u32, 0, no.as_str()).unwrap();
            ws.write(ri as u32, 1, old.as_str()).unwrap();
            if old == "notes.TXT" { continue; } // 留空跳过
            let nb = format!("新_{}", Path::new(&old).file_stem().unwrap().to_str().unwrap());
            ws.write(ri as u32, 2, nb.as_str()).unwrap();
        }
        w.save(d.join(LIST_FILE_NAME)).unwrap();

        let r = rename(&d.join(LIST_FILE_NAME), "{no}_PRE_{new}.{ext}").unwrap();
        assert!(r.problems.is_empty() && r.failures.is_empty(), "{:?} {:?}", r.problems, r.failures);
        assert_eq!(r.done, seed.len() - 1);
        let names = names_in(&d);
        assert!(names.iter().any(|n| n.ends_with("_PRE_新_旅游 记录.mp4")), "{:?}", names);
        assert!(names.iter().any(|n| n.ends_with("_PRE_新_风景.jpg")));
        assert!(names.contains(&"notes.TXT".to_string()), "留空跳过");
        assert!(names.contains(&LIST_FILE_NAME.to_string()), "清单保留");

        // 冲突拦截
        let d2 = tmpdir();
        fs::write(d2.join("old.txt"), b"a").unwrap();
        fs::write(d2.join("target.txt"), b"b").unwrap();
        let list2 = d2.join(LIST_FILE_NAME);
        write_sheet(&list2, &["no", "old", "new"], &[
            vec!["1".into(), "old.txt".into(), "target".into()],
            vec!["2".into(), "target.txt".into(), String::new()],
        ]);
        let c = rename(&list2, "{new}.{ext}").unwrap();
        assert!(c.problems.iter().any(|p| p.contains("目标文件已存在")), "{:?}", c.problems);
        assert!(d2.join("old.txt").is_file(), "校验失败不改源文件");

        let _ = fs::remove_dir_all(&d);
        let _ = fs::remove_dir_all(&d2);
    }

    /// 功能二独立性：清单可任意命名，处理范围 = 清单所在文件夹。
    #[test]
    fn rename_with_arbitrary_list_name() {
        let d = tmpdir();
        fs::write(d.join("甲.TXT"), b"a").unwrap();
        fs::write(d.join("乙.JPG"), b"b").unwrap();

        let custom_list = d.join("我的自定义清单.xlsx");
        write_sheet(&custom_list, &["no", "old", "new"], &[
            vec!["1".into(), "甲.TXT".into(), "新名字".into()],
            vec!["2".into(), "乙.JPG".into(), "照片".into()],
        ]);

        let r = rename(&custom_list, "{no}_{new}.{ext}").unwrap();
        assert!(r.problems.is_empty() && r.failures.is_empty(), "{:?} {:?}", r.problems, r.failures);
        assert_eq!(r.done, 2);
        assert!(d.join("1_新名字.txt").is_file(), "后缀小写");
        assert!(d.join("2_照片.jpg").is_file());
        assert!(custom_list.is_file(), "清单保留");
        let _ = fs::remove_dir_all(&d);
    }

    /// 表头即变量：自定义列 + 内置 old/ext/hash + 未定义变量提示。
    #[test]
    fn template_variables() {
        // 自定义列变量
        let d = tmpdir();
        fs::write(d.join("音乐.MP3"), b"a").unwrap();
        let list = d.join("清单.xlsx");
        write_sheet(&list, &["no", "old", "new", "作者"], &[
            vec!["1".into(), "音乐.MP3".into(), "歌曲".into(), "张三".into()],
        ]);
        let r = rename(&list, "{作者}-{new}.{ext}").unwrap();
        assert!(r.problems.is_empty() && r.failures.is_empty(), "{:?} {:?}", r.problems, r.failures);
        assert!(d.join("张三-歌曲.mp3").is_file());
        let _ = fs::remove_dir_all(&d);

        // 内置变量：old 剥离后缀、ext 小写、hash 完整 MD5（内容 "a" → 0cc175b9...）
        let d2 = tmpdir();
        fs::write(d2.join("测试.TXT"), b"a").unwrap();
        let list2 = d2.join("清单.xlsx");
        write_sheet(&list2, &["no", "old", "new"], &[
            vec!["1".into(), "测试.TXT".into(), String::new()],
        ]);
        let r = rename(&list2, "{old}-x.{ext}").unwrap();
        assert!(d2.join("测试-x.txt").is_file(), "模板不含 {{new}} 时留空也处理，且 old 剥离后缀、ext 小写");
        let _ = fs::remove_dir_all(&d2);

        let d3 = tmpdir();
        fs::write(d3.join("文件.TXT"), b"a").unwrap();
        let list3 = d3.join("清单.xlsx");
        write_sheet(&list3, &["no", "old", "new"], &[
            vec!["1".into(), "文件.TXT".into(), String::new()],
        ]);
        let r = rename(&list3, "{hash}.{ext}").unwrap();
        assert!(d3.join("0cc175b9c0f1b6a831c399e269772661.txt").is_file(), "完整小写 MD5");
        let _ = fs::remove_dir_all(&d3);

        // 未定义变量：替换为空 + 收集名字
        let d4 = tmpdir();
        fs::write(d4.join("文件.TXT"), b"a").unwrap();
        let list4 = d4.join("清单.xlsx");
        write_sheet(&list4, &["no", "old", "new"], &[
            vec!["1".into(), "文件.TXT".into(), "名字".into()],
        ]);
        let r = rename(&list4, "{new}_{date}.{ext}").unwrap();
        assert_eq!(r.unknown, vec!["date".to_string()]);
        assert!(d4.join("名字_.txt").is_file());
        let _ = fs::remove_dir_all(&d4);
    }

    /// {no} 是内置变量：按实际处理行从 1 重新编号，忽略表格 no 列的任意值；
    /// 中间空行（old 为空）不占号。
    #[test]
    fn no_is_renumbered() {
        let d = tmpdir();
        fs::write(d.join("甲.TXT"), b"a").unwrap();
        fs::write(d.join("乙.TXT"), b"b").unwrap();
        fs::write(d.join("丙.TXT"), b"c").unwrap();
        let list = d.join("清单.xlsx");
        // no 列故意写成 99/50/7，且中间夹一个 old 为空的行
        write_sheet(&list, &["no", "old", "new"], &[
            vec!["99".into(), "甲.TXT".into(), "一".into()],
            vec!["50".into(), String::new(), String::new()],
            vec!["7".into(), "乙.TXT".into(), "二".into()],
            vec!["0".into(), "丙.TXT".into(), "三".into()],
        ]);
        let r = rename(&list, "{no}-{new}.{ext}").unwrap();
        assert!(r.problems.is_empty() && r.failures.is_empty(), "{:?} {:?}", r.problems, r.failures);
        assert_eq!(r.done, 3);
        assert!(d.join("1-一.txt").is_file(), "首行应为 1，忽略表格里的 99");
        assert!(d.join("2-二.txt").is_file(), "空行不占号，次行应为 2");
        assert!(d.join("3-三.txt").is_file());
        let _ = fs::remove_dir_all(&d);
    }
}
