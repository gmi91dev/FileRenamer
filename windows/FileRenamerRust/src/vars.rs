//! 模板变量计算与渲染。
//! 依据：微软文档 GetFileTime / FileTimeToSystemTime / GetSystemTimeAsFileTime。

use std::collections::HashMap;
use std::fs::File;
use std::os::windows::io::AsRawHandle;
use std::path::Path;

use windows::Win32::Foundation::{FILETIME, HANDLE, SYSTEMTIME};
use windows::Win32::Storage::FileSystem::GetFileTime;
use windows::Win32::System::SystemInformation::GetSystemTimeAsFileTime;
use windows::Win32::System::Time::FileTimeToSystemTime;

pub struct RenderResult {
    pub name: String,
    pub unknown: Vec<String>,
}

/// 把模板中的 {变量} 替换为对应值；未定义变量替换为空串并收集名字。
/// 支持 {{ 和 }} 作为字面量转义。
pub fn render(template: &str, values: &HashMap<String, String>) -> RenderResult {
    let chars: Vec<char> = template.chars().collect();
    let mut out = String::new();
    let mut unknown: Vec<String> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' && i + 1 < chars.len() {
            if chars[i + 1] == '{' {
                out.push('{');
                i += 2;
                continue;
            }
            if let Some(offset) = chars[i + 1..].iter().position(|&c| c == '}') {
                let key: String = chars[i + 1..i + 1 + offset].iter().collect();
                if let Some(value) = values.get(&key) {
                    out.push_str(value);
                } else if !unknown.contains(&key) {
                    unknown.push(key);
                }
                i += offset + 2;
                continue;
            }
        }
        if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
            out.push('}');
            i += 2;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    RenderResult { name: out, unknown }
}

/// 人类可读文件大小。
pub fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / KB)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (KB * KB))
    } else {
        format!("{:.1} GB", bytes as f64 / (KB * KB * KB))
    }
}

/// 完整 MD5（32 位小写十六进制）。依据 docs.rs/md5：format!("{:x}", md5::compute(data))。
pub fn file_md5(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(data) => format!("{:x}", md5::compute(&data)),
        Err(_) => String::new(),
    }
}

unsafe fn date_of(ft: &FILETIME) -> Option<(u16, u16, u16)> {
    let mut st = SYSTEMTIME::default();
    FileTimeToSystemTime(ft, &mut st).ok()?;
    Some((st.wYear, st.wMonth, st.wDay))
}

/// 读取文件的创建日期与最后修改日期，返回 (年,月,日)。失败时返回 (None, None)。
pub unsafe fn file_dates(path: &Path) -> (Option<(u16, u16, u16)>, Option<(u16, u16, u16)>) {
    let Ok(file) = File::open(path) else { return (None, None) };
    let handle = HANDLE(file.as_raw_handle() as *mut core::ffi::c_void);
    let mut creation = FILETIME::default();
    let mut last_write = FILETIME::default();
    if GetFileTime(handle, Some(&mut creation), None, Some(&mut last_write)).is_err() {
        return (None, None);
    }
    (date_of(&creation), date_of(&last_write))
}

/// 执行重命名时的当前日期。
pub unsafe fn today() -> Option<(u16, u16, u16)> {
    date_of(&GetSystemTimeAsFileTime())
}

pub fn format_date(y: u16, m: u16, d: u16) -> String {
    format!("{:04}-{:02}-{:02}", y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_basics() {
        let mut values = HashMap::new();
        values.insert("new".to_string(), "旅游记录".to_string());
        values.insert("ext".to_string(), "mp4".to_string());
        values.insert("no".to_string(), "1".to_string());

        let r = render("{no}_{new}.{ext}", &values);
        assert_eq!(r.name, "1_旅游记录.mp4");
        assert!(r.unknown.is_empty());

        let r = render("{new} {date}.{ext}", &values);
        assert_eq!(r.name, "旅游记录 .mp4");
        assert_eq!(r.unknown, vec!["date".to_string()]);

        let r = render("{{{new}}}", &values);
        assert_eq!(r.name, "{旅游记录}");
    }
}
