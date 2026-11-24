#![windows_subsystem = "windows"]

mod logic;
mod vars;

use std::sync::atomic::{AtomicI32, AtomicIsize, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, COLOR_WINDOW, CreateFontIndirectW, HBRUSH, HFONT, HDC, NULL_BRUSH,
    TRANSPARENT, GetStockObject, SetBkMode, GetMonitorInfoW, MonitorFromWindow,
    MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    BIF_EDITBOX, BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW,
    SHBrowseForFolderW, SHGetPathFromIDListW,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::InitCommonControls;
use windows::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OPENFILENAMEW, OFN_EXPLORER, OFN_FILEMUSTEXIST,
    OFN_HIDEREADONLY, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST,
};
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::{
    BM_GETCHECK, BN_CLICKED, BS_AUTOCHECKBOX, BS_GROUPBOX, BS_PUSHBUTTON,
    CreateWindowExW, DefWindowProcW, DispatchMessageW, ES_AUTOHSCROLL, ES_READONLY,
    GetMessageW, GetWindowTextLengthW, GetWindowTextW, MessageBoxW,
    PostQuitMessage, RegisterClassW, SendMessageW, SetWindowPos, SetWindowTextW, GetWindowRect,
    SystemParametersInfoW, TranslateMessage, WM_COMMAND, WM_CTLCOLORSTATIC, WM_DESTROY, WM_SETFONT,
    WS_BORDER, WS_CHILD, WS_OVERLAPPEDWINDOW, WS_VISIBLE, WNDCLASSW, MB_ICONERROR, MB_ICONINFORMATION,
    MB_ICONWARNING, MB_OK, CW_USEDEFAULT, HMENU, IDC_ARROW, LoadCursorW, ShowWindow,
    SW_SHOW, WINDOW_STYLE, SPI_GETNONCLIENTMETRICS, NONCLIENTMETRICSW,
    SWP_NOSIZE, SWP_NOZORDER,
};

// 标准 Win32 宏语义（windows crate 不封装这些 C 宏）
// LOWORD/HIWORD 见 Winuser.h：LOWORD(x)=低16位, HIWORD(x)=高16位
fn hiword(v: u32) -> u16 { (v >> 16) as u16 }
const BST_CHECKED_VALUE: isize = 0x0001; // WinUser.h: #define BST_CHECKED 0x0001
use windows::core::{HSTRING, PCWSTR, PWSTR, w};

// 控件句柄（单线程窗口过程使用原子值保存）
static H_FOLDER: AtomicIsize = AtomicIsize::new(0);
static H_LIST: AtomicIsize = AtomicIsize::new(0);
static H_PREFIX: AtomicIsize = AtomicIsize::new(0);
static H_VIDEO: AtomicIsize = AtomicIsize::new(0);
static H_IMAGE: AtomicIsize = AtomicIsize::new(0);
static H_DOC: AtomicIsize = AtomicIsize::new(0);
static H_MAIN: AtomicIsize = AtomicIsize::new(0);
static H_FONT: AtomicIsize = AtomicIsize::new(0);
// DPI 缩放百分比（100 = 96dpi），PerMonitorV2 下坐标按此缩放
static SCALE_PCT: AtomicI32 = AtomicI32::new(100);

const IDC_TXT_FOLDER: i32 = 1001;
const IDC_BTN_BROWSE: i32 = 1002;
const IDC_CHK_VIDEO: i32 = 1003;
const IDC_CHK_IMAGE: i32 = 1004;
const IDC_CHK_DOC: i32 = 1005;
const IDC_BTN_GENERATE: i32 = 1006;
const IDC_TXT_PREFIX: i32 = 1007;
const IDC_BTN_RENAME: i32 = 1008;
const IDC_TXT_LIST: i32 = 1009;
const IDC_BTN_BROWSE_LIST: i32 = 1010;
const IDC_BTN_VARS: i32 = 1011;

/// 变量说明弹窗文本。表头即变量：表格第一行列名可作 {变量}；以下为内置变量。
fn vars_help_text() -> String {
    [
        "【内置变量】（表格第一行的列名也可直接当作 {变量} 使用）",
        "",
        "no       序号，按实际处理行从 1 编号；空行不占号，忽略表格 no 列的值",
        "old      旧文件名主名，不含后缀",
        "new      表格 new 列填写的新名称",
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
        "3. 表格右侧可加自定义列，表头写什么就能用 {什么}，例如列名“作者”→ {作者}。",
        "4. 模板中写了不存在且非内置的变量（如 {date}），会替换为空并在完成时列出提醒。",
        "5. {no} 按实际处理的行重新编号、跳过空行，因此可能与原表格行号不同。",
        "6. 新名称不能含 \\ / : * ? \" < > | 等字符；后缀统一输出小写。",
    ].join("\n")
}

/// 把顶层窗口移动到当前显示器工作区中央。
/// 依据：微软文档 GetWindowRect / MonitorFromWindow / GetMonitorInfoW / SetWindowPos。
/// rcWork 为排除任务栏后的工作区；PerMonitorV2 下坐标均为物理像素。
unsafe fn center_window(hwnd: HWND) {
    let mut rc = RECT::default();
    if GetWindowRect(hwnd, &mut rc).is_err() { return; }
    let w = rc.right - rc.left;
    let h = rc.bottom - rc.top;

    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(monitor, &mut mi).as_bool() { return; }
    let work = mi.rcWork;
    let x = work.left + (work.right - work.left - w) / 2;
    let y = work.top + (work.bottom - work.top - h) / 2;
    let _ = SetWindowPos(hwnd, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
}

fn main() {
    unsafe {
        // 微软文档 "Enabling Visual Styles"：声明 ComCtl32 v6 manifest 后须调用
        // InitCommonControls，标准控件（含复选框按钮）才会用主题绘制，去掉灰底。
        InitCommonControls();

        let instance = GetModuleHandleW(None).unwrap();
        let class_name = w!("FileRenamerRustWindow");

        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: class_name,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            // 系统颜色刷需用 (COLOR_x + 1) 作为句柄值（Win32 惯例）
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as *mut core::ffi::c_void),
            ..Default::default()
        };
        RegisterClassW(&wc);

        // PerMonitorV2 下坐标是物理像素，需按实际 DPI 缩放布局（基准 96dpi）。
        let dpi = GetDpiForSystem();
        let scale_pct = ((dpi as f32 / 96.0) * 100.0).round() as i32;
        SCALE_PCT.store(scale_pct, Ordering::SeqCst);

        // 采用系统当前 UI 字体（中文 Windows 上是微软雅黑）+ ClearType 抗锯齿，
        // 替代 Win32 控件的默认位图字体（小、发虚、无抗锯齿 → 显得"黑"）。
        if let Some(font) = create_ui_font() {
            H_FONT.store(font.0 as isize, Ordering::SeqCst);
        }

        let hwnd = CreateWindowExW(
            Default::default(),
            class_name,
            w!("批量重命名工具"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            sc(496),
            sc(344),
            None,
            None,
            Some(instance.into()),
            None,
        ).expect("create main window");
        H_MAIN.store(hwnd.0 as isize, Ordering::SeqCst);

        build_controls(hwnd);
        center_window(hwnd);
        ShowWindow(hwnd, SW_SHOW);

        let mut msg = std::mem::zeroed();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

// 读取系统当前 UI 字体并强制 ClearType，返回可应用于控件的逻辑字体。
// 依据：微软文档 SystemParametersInfoW(SPI_GETNONCLIENTMETRICS) 的 lfMessageFont
// 即系统 UI 文本字体；CreateFontIndirectW + lfQuality=CLEARTYPE_QUALITY 启用抗锯齿。
unsafe fn create_ui_font() -> Option<HFONT> {
    let mut ncm: NONCLIENTMETRICSW = std::mem::zeroed();
    ncm.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
    if SystemParametersInfoW(
        SPI_GETNONCLIENTMETRICS,
        ncm.cbSize,
        Some(&mut ncm as *mut _ as *mut core::ffi::c_void),
        Default::default(),
    ).is_ok() {
        ncm.lfMessageFont.lfQuality = CLEARTYPE_QUALITY;
        let font = CreateFontIndirectW(&ncm.lfMessageFont);
        if !font.0.is_null() { return Some(font); }
    }
    None
}

unsafe fn build_controls(parent: HWND) {
    let base = WS_CHILD.0 | WS_VISIBLE.0;
    let group = WINDOW_STYLE(BS_GROUPBOX as u32 | base);
    let label = WINDOW_STYLE(base);
    let button = WINDOW_STYLE(BS_PUSHBUTTON as u32 | base);
    let checkbox = WINDOW_STYLE(BS_AUTOCHECKBOX as u32 | base);
    let edit = WINDOW_STYLE(WS_BORDER.0 | ES_AUTOHSCROLL as u32 | base);

    make(w!("BUTTON"), w!("功能一：快速生成清单（快捷操作）"), group, 12, 12, 456, 148, parent, 0);
    make(w!("STATIC"), w!("文件夹："), label, 26, 36, 60, 18, parent, 0);
    let h = make(w!("EDIT"), w!(""), WINDOW_STYLE(edit.0 | ES_READONLY as u32), 90, 32, 282, 22, parent, IDC_TXT_FOLDER);
    H_FOLDER.store(h.0 as isize, Ordering::SeqCst);
    make(w!("BUTTON"), w!("浏览…"), button, 380, 30, 78, 26, parent, IDC_BTN_BROWSE);
    make(w!("STATIC"), w!("文件类型（都不选 = 全部）："), label, 26, 66, 210, 18, parent, 0);
    let h = make(w!("BUTTON"), w!("视频"), checkbox, 28, 92, 60, 20, parent, IDC_CHK_VIDEO);
    H_VIDEO.store(h.0 as isize, Ordering::SeqCst);
    let h = make(w!("BUTTON"), w!("图片"), checkbox, 96, 92, 60, 20, parent, IDC_CHK_IMAGE);
    H_IMAGE.store(h.0 as isize, Ordering::SeqCst);
    let h = make(w!("BUTTON"), w!("文档"), checkbox, 164, 92, 60, 20, parent, IDC_CHK_DOC);
    H_DOC.store(h.0 as isize, Ordering::SeqCst);
    make(w!("BUTTON"), w!("生成清单表格"), button, 330, 88, 126, 28, parent, IDC_BTN_GENERATE);
    make(w!("STATIC"), w!("生成位置：所选文件夹内的 重命名清单.xlsx"), label, 26, 124, 320, 18, parent, 0);

    // 功能二与功能一完全独立：清单文件手动选择，处理范围 = 清单所在文件夹
    make(w!("BUTTON"), w!("功能二：按清单重命名（独立功能）"), group, 12, 172, 456, 160, parent, 0);
    make(w!("STATIC"), w!("清单文件："), label, 26, 196, 80, 18, parent, 0);
    let h = make(w!("EDIT"), w!(""), WINDOW_STYLE(edit.0 | ES_READONLY as u32), 106, 192, 266, 22, parent, IDC_TXT_LIST);
    H_LIST.store(h.0 as isize, Ordering::SeqCst);
    make(w!("BUTTON"), w!("浏览…"), button, 380, 190, 78, 26, parent, IDC_BTN_BROWSE_LIST);
    make(w!("STATIC"), w!("名称格式："), label, 26, 228, 90, 18, parent, 0);
    let h = make(w!("EDIT"), w!("{new}.{ext}"), edit, 116, 224, 200, 22, parent, IDC_TXT_PREFIX);
    H_PREFIX.store(h.0 as isize, Ordering::SeqCst);
    make(w!("BUTTON"), w!("读取清单并重命名"), button, 330, 222, 126, 28, parent, IDC_BTN_RENAME);
    make(w!("STATIC"), w!("表头即变量（详见右侧按钮）"), label, 26, 262, 220, 18, parent, 0);
    make(w!("BUTTON"), w!("查看变量说明"), button, 330, 256, 126, 28, parent, IDC_BTN_VARS);
}

// 把 96dpi 基准坐标换算为当前 DPI 下的物理像素
fn sc(v: i32) -> i32 {
    (v * SCALE_PCT.load(Ordering::SeqCst) / 100)
}

unsafe fn make(class: PCWSTR, text: PCWSTR, style: WINDOW_STYLE,
               x: i32, y: i32, w: i32, h: i32, parent: HWND, id: i32) -> HWND {
    let hwnd = CreateWindowExW(
        Default::default(), class, text,
        style, sc(x), sc(y), sc(w), sc(h),
        Some(parent),
        if id == 0 { None } else { Some(HMENU(id as *mut core::ffi::c_void)) },
        None, None,
    ).expect("create control");
    apply_font(hwnd);
    hwnd
}

// 依据 WM_SETFONT 文档：wParam=HFONT，lParam=TRUE 表示立即重绘。
unsafe fn apply_font(hwnd: HWND) {
    let h = H_FONT.load(Ordering::SeqCst);
    if h != 0 {
        let _ = SendMessageW(hwnd, WM_SETFONT, Some(WPARAM(h as usize)), Some(LPARAM(1)));
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let v = wparam.0 as u32;
            let id = (v & 0xffff) as i32;              // LOWORD
            let code = hiword(v);
            if code == BN_CLICKED as u16 {
                match id {
                    IDC_BTN_BROWSE => on_browse(),
                    IDC_BTN_GENERATE => on_generate(),
                    IDC_BTN_BROWSE_LIST => on_browse_list(),
                    IDC_BTN_RENAME => on_rename(),
                    IDC_BTN_VARS => msg_box(&vars_help_text(), "变量说明", MB_OK | MB_ICONINFORMATION),
                    _ => {}
                }
            }
            LRESULT(0)
        }
        // 依据微软文档 WM_CTLCOLORSTATIC：只读 EDIT 也会发此消息。
        // 对只读 EDIT 必须交给 DefWindowProc 用系统默认颜色画背景，
        // 否则返回 NULL_BRUSH 会导致背景不清除 → 按钮文字残影 + 黑色选区。
        WM_CTLCOLORSTATIC => {
            let ctrl = HWND(lparam.0 as *mut core::ffi::c_void);
            if ctrl == hwnd_of(&H_FOLDER) || ctrl == hwnd_of(&H_LIST) {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            } else {
                // 仅对 STATIC 标签：文字背景透明，消除灰色方块。
                let hdc = HDC(wparam.0 as *mut core::ffi::c_void);
                SetBkMode(hdc, TRANSPARENT);
                LRESULT(GetStockObject(NULL_BRUSH).0 as isize)
            }
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn hwnd_of(atom: &AtomicIsize) -> HWND { HWND(atom.load(Ordering::SeqCst) as *mut core::ffi::c_void) }

unsafe fn edit_text(h: HWND) -> String {
    let len = GetWindowTextLengthW(h);
    if len <= 0 { return String::new(); }
    let mut buf = vec![0u16; (len + 1) as usize];
    GetWindowTextW(h, &mut buf);
    String::from_utf16_lossy(&buf[..len as usize])
}

unsafe fn is_checked(h: HWND) -> bool {
    SendMessageW(h, BM_GETCHECK, Some(WPARAM::default()), Some(LPARAM::default())).0 == BST_CHECKED_VALUE
}

fn msg_box(text: &str, title: &str, flags: windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE) {
    let t = HSTRING::from(text);
    let c = HSTRING::from(title);
    unsafe { MessageBoxW(None, &t, &c, flags); }
}

unsafe fn on_browse() {
    let mut display = [0u16; 260];
    let bi = BROWSEINFOW {
        hwndOwner: hwnd_of(&H_MAIN),
        lpszTitle: w!("选择要处理的文件夹"),
        pszDisplayName: PWSTR(display.as_mut_ptr()),
        ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE | BIF_EDITBOX,
        ..Default::default()
    };
    let pidl = SHBrowseForFolderW(&bi);
    if !pidl.is_null() {
        let mut path = [0u16; 260];
        if SHGetPathFromIDListW(pidl, &mut path).as_bool() {
            let s = String::from_utf16_lossy(&path);
            let s = s.trim_end_matches('\0');
            SetWindowTextW(hwnd_of(&H_FOLDER), &HSTRING::from(s)).ok();
        }
        CoTaskMemFree(Some(pidl as *const core::ffi::c_void));
    }
}

/// 功能二：手动选择清单 xlsx 文件。
/// 依据：微软文档 GetOpenFileNameW / OPENFILENAMEW（Win32_UI_Controls_Dialogs feature）。
unsafe fn on_browse_list() {
    let mut file = [0u16; 1024];
    let current = edit_text(hwnd_of(&H_LIST));
    if !current.is_empty() {
        let wide: Vec<u16> = current.encode_utf16().chain(std::iter::once(0)).collect();
        let n = wide.len().min(file.len());
        file[..n].copy_from_slice(&wide[..n]);
    }

    let mut ofn: OPENFILENAMEW = std::mem::zeroed();
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = hwnd_of(&H_MAIN);
    // 过滤器是成对 NUL 分隔字符串，w! 宏在末尾再补一个 NUL → 双 NUL 结尾
    ofn.lpstrFilter = w!("Excel 工作簿 (*.xlsx)\0*.xlsx\0所有文件 (*.*)\0*.*\0");
    ofn.lpstrFile = PWSTR(file.as_mut_ptr());
    ofn.nMaxFile = file.len() as u32;
    ofn.lpstrTitle = w!("选择重命名清单文件");
    ofn.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY | OFN_NOCHANGEDIR;
    ofn.lpstrDefExt = w!("xlsx");

    if GetOpenFileNameW(&mut ofn).as_bool() {
        let s = String::from_utf16_lossy(&file);
        let s = s.trim_end_matches('\0');
        SetWindowTextW(hwnd_of(&H_LIST), &HSTRING::from(s)).ok();
    }
}

unsafe fn on_generate() {
    let folder = unsafe { edit_text(hwnd_of(&H_FOLDER)) };
    let folder = folder.trim();
    if folder.is_empty() || !std::path::Path::new(folder).is_dir() {
        msg_box("请先选择有效的文件夹。", "提示", MB_OK | MB_ICONINFORMATION);
        return;
    }
    let filter = logic::TypeFilter {
        video: unsafe { is_checked(hwnd_of(&H_VIDEO)) },
        image: unsafe { is_checked(hwnd_of(&H_IMAGE)) },
        doc: unsafe { is_checked(hwnd_of(&H_DOC)) },
    };
    match logic::generate_list(std::path::Path::new(folder), &filter) {
        Ok(names) => {
            let list = std::path::Path::new(folder).join(logic::LIST_FILE_NAME);
            msg_box(&format!("已生成清单：{}\n共 {} 个文件。\n填好第二列后，可在功能二中随时处理。", list.display(), names.len()),
                    "完成", MB_OK | MB_ICONINFORMATION);
        }
        Err(e) => msg_box(&format!("生成清单失败：\n{}", e), "错误", MB_OK | MB_ICONERROR),
    }
}

unsafe fn on_rename() {
    let list_text = edit_text(hwnd_of(&H_LIST));
    let list_text = list_text.trim();
    if list_text.is_empty() {
        msg_box("请先选择清单文件。", "提示", MB_OK | MB_ICONINFORMATION);
        return;
    }
    let list = std::path::Path::new(list_text);
    if !list.is_file() {
        msg_box(&format!("未找到清单文件：\n{}", list.display()), "提示", MB_OK | MB_ICONINFORMATION);
        return;
    }
    let template = edit_text(hwnd_of(&H_PREFIX));
    if template.trim().is_empty() {
        msg_box("请填写名称格式，例如 {new}.{ext}", "提示", MB_OK | MB_ICONINFORMATION);
        return;
    }
    match logic::rename(list, &template) {
        Ok(r) => {
            let unknown_hint = if r.unknown.is_empty() { String::new() } else {
                format!("\n\n未定义变量（已按空值处理）：{}", r.unknown.join("、"))
            };
            if !r.problems.is_empty() {
                let mut s = format!("发现以下问题，未执行任何重命名：\n\n{}\n", r.problems[..r.problems.len().min(20)].join("\n"));
                if r.problems.len() > 20 { s.push_str(&format!("\n……等共 {} 项", r.problems.len())); }
                s.push_str(&unknown_hint);
                msg_box(&s, "无法继续", MB_OK | MB_ICONWARNING);
            } else if r.failures.is_empty() {
                msg_box(&format!("重命名完成，共处理 {} 个文件。{}", r.done, unknown_hint),
                    "完成", MB_OK | MB_ICONINFORMATION);
            } else {
                msg_box(&format!("完成 {} 个，失败 {} 个：\n\n{}{}", r.done, r.failures.len(),
                    r.failures[..r.failures.len().min(20)].join("\n"), unknown_hint), "部分失败", MB_OK | MB_ICONWARNING);
            }
        }
        Err(e) => msg_box(&format!("重命名失败：\n{}", e), "错误", MB_OK | MB_ICONERROR),
    }
}
