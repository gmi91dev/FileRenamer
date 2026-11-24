#![windows_subsystem = "windows"]

mod logic;
mod vars;

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicU32, Ordering};

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, COLOR_WINDOW, CreateFontIndirectW, CreatePen, DeleteObject, HBRUSH, HFONT,
    HDC, NULL_BRUSH, PS_SOLID, TRANSPARENT, Arc, BeginPaint, EndPaint, GetStockObject, InvalidateRect,
    SelectObject, SetBkMode, PAINTSTRUCT, GetMonitorInfoW, MonitorFromWindow,
    MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    BIF_EDITBOX, BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW,
    SHBrowseForFolderW, SHGetPathFromIDListW,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
// 依据 windows crate 源码：EnableWindow 归属 KeyboardAndMouse 模块（对应 winuser.h 实现）
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::Controls::InitCommonControls;
use windows::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OPENFILENAMEW, OFN_EXPLORER, OFN_FILEMUSTEXIST,
    OFN_HIDEREADONLY, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST,
};
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::{
    BM_GETCHECK, BN_CLICKED, BS_AUTOCHECKBOX, BS_GROUPBOX, BS_PUSHBUTTON,
    CreateWindowExW, DefWindowProcW, DispatchMessageW, ES_AUTOHSCROLL, ES_READONLY,
    GetMessageW, GetClientRect, GetWindowTextLengthW, GetWindowTextW, MessageBoxW,
    HTCLIENT, HWND_TOP, IDC_WAIT, KillTimer, PostMessageW, PostQuitMessage, RegisterClassW,
    SendMessageW, SetCursor, SetTimer, SetWindowPos, SetWindowTextW, GetWindowRect,
    SystemParametersInfoW, TranslateMessage, WM_APP, WM_COMMAND, WM_CTLCOLORSTATIC, WM_SETCURSOR,
    WM_TIMER, WM_PAINT, WM_SETFONT, WM_DESTROY,
    WS_BORDER, WS_CHILD, WS_OVERLAPPEDWINDOW, WS_VISIBLE, WNDCLASSW, MB_ICONERROR, MB_ICONINFORMATION,
    MB_ICONWARNING, MB_OK, CW_USEDEFAULT, HMENU, IDC_ARROW, LoadCursorW, ShowWindow,
    SW_HIDE, SW_SHOW, SWP_SHOWWINDOW, WINDOW_STYLE, SPI_GETNONCLIENTMETRICS, NONCLIENTMETRICSW,
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
// 忙碌期间要禁用的四个操作按钮：浏览×2、生成清单、重命名
static H_BTN_BROWSE: AtomicIsize = AtomicIsize::new(0);
static H_BTN_BROWSE_LIST: AtomicIsize = AtomicIsize::new(0);
static H_BTN_GENERATE: AtomicIsize = AtomicIsize::new(0);
static H_BTN_RENAME: AtomicIsize = AtomicIsize::new(0);
// 旋转加载指示器（菊花）子窗口
static H_SPIN: AtomicIsize = AtomicIsize::new(0);
// 是否有后台任务执行中（用于禁用按钮与 WM_SETCURSOR 等待光标）
static BUSY: AtomicBool = AtomicBool::new(false);
// 菊花当前角度（度），WM_TIMER 递增
static SPIN_ANGLE: AtomicU32 = AtomicU32::new(0);
// DPI 缩放百分比（100 = 96dpi），PerMonitorV2 下坐标按此缩放
static SCALE_PCT: AtomicI32 = AtomicI32::new(100);

// 自定义消息：后台线程完成后 PostMessageW 回 UI 线程（WM_APP 段供应用自定义）
const WM_WORKDONE: u32 = WM_APP + 1;
// 菊花动画参数：每 40ms 转 24°，约 600ms 一圈
const SPIN_TIMER_ID: usize = 1;
const SPIN_INTERVAL_MS: u32 = 40;
const SPIN_STEP_DEG: u32 = 24;
const SPIN_SIZE: i32 = 36; // 96dpi 基准边长

/// 后台任务结果：worker 线程 Box::into_raw 后经 WM_WORKDONE 的 lParam 传回 UI 线程。
/// 错误统一转成 String（io::Error 跨线程显示用）。
enum WorkResult {
    /// (生成结果：Ok(文件数)，清单完整路径文本)
    Generate(Result<usize, String>, String),
    Rename(Result<logic::RenameResult, String>),
}

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

        // 菊花子窗口类：与主窗口同底色，WM_PAINT 自绘旋转圆弧
        let spin_class = w!("FileRenamerSpinner");
        let swc = WNDCLASSW {
            lpfnWndProc: Some(spinner_proc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as *mut core::ffi::c_void),
            lpszClassName: spin_class,
            ..Default::default()
        };
        RegisterClassW(&swc);

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
    let h = make(w!("BUTTON"), w!("浏览…"), button, 380, 30, 78, 26, parent, IDC_BTN_BROWSE);
    H_BTN_BROWSE.store(h.0 as isize, Ordering::SeqCst);
    make(w!("STATIC"), w!("文件类型（都不选 = 全部）："), label, 26, 66, 210, 18, parent, 0);
    let h = make(w!("BUTTON"), w!("视频"), checkbox, 28, 92, 60, 20, parent, IDC_CHK_VIDEO);
    H_VIDEO.store(h.0 as isize, Ordering::SeqCst);
    let h = make(w!("BUTTON"), w!("图片"), checkbox, 96, 92, 60, 20, parent, IDC_CHK_IMAGE);
    H_IMAGE.store(h.0 as isize, Ordering::SeqCst);
    let h = make(w!("BUTTON"), w!("文档"), checkbox, 164, 92, 60, 20, parent, IDC_CHK_DOC);
    H_DOC.store(h.0 as isize, Ordering::SeqCst);
    let h = make(w!("BUTTON"), w!("生成清单表格"), button, 330, 88, 126, 28, parent, IDC_BTN_GENERATE);
    H_BTN_GENERATE.store(h.0 as isize, Ordering::SeqCst);
    make(w!("STATIC"), w!("生成位置：所选文件夹内的 重命名清单.xlsx"), label, 26, 124, 320, 18, parent, 0);

    // 功能二与功能一完全独立：清单文件手动选择，处理范围 = 清单所在文件夹
    make(w!("BUTTON"), w!("功能二：按清单重命名（独立功能）"), group, 12, 172, 456, 160, parent, 0);
    make(w!("STATIC"), w!("清单文件："), label, 26, 196, 80, 18, parent, 0);
    let h = make(w!("EDIT"), w!(""), WINDOW_STYLE(edit.0 | ES_READONLY as u32), 106, 192, 266, 22, parent, IDC_TXT_LIST);
    H_LIST.store(h.0 as isize, Ordering::SeqCst);
    let h = make(w!("BUTTON"), w!("浏览…"), button, 380, 190, 78, 26, parent, IDC_BTN_BROWSE_LIST);
    H_BTN_BROWSE_LIST.store(h.0 as isize, Ordering::SeqCst);
    make(w!("STATIC"), w!("名称格式："), label, 26, 228, 90, 18, parent, 0);
    let h = make(w!("EDIT"), w!("{new}.{ext}"), edit, 116, 224, 200, 22, parent, IDC_TXT_PREFIX);
    H_PREFIX.store(h.0 as isize, Ordering::SeqCst);
    let h = make(w!("BUTTON"), w!("读取清单并重命名"), button, 330, 222, 126, 28, parent, IDC_BTN_RENAME);
    H_BTN_RENAME.store(h.0 as isize, Ordering::SeqCst);
    make(w!("STATIC"), w!("表头即变量（详见右侧按钮）"), label, 26, 262, 220, 18, parent, 0);
    make(w!("BUTTON"), w!("查看变量说明"), button, 330, 256, 126, 28, parent, IDC_BTN_VARS);

    // 菊花子窗口：默认隐藏，忙碌时居中显示在主窗口上层。
    // 自定义窗口类按「类名 + hInstance」定位，hInstance 必须传注册时的模块句柄
    // （系统类 BUTTON/EDIT 传 None 没问题，但自注册类传 None 会找不到类导致创建失败）。
    let instance = GetModuleHandleW(None).expect("get module handle");
    let spin = CreateWindowExW(
        Default::default(),
        w!("FileRenamerSpinner"),
        w!(""),
        WINDOW_STYLE(WS_CHILD.0),
        sc(12), sc(12), sc(SPIN_SIZE), sc(SPIN_SIZE),
        Some(parent),
        None,
        Some(instance.into()),
        None,
    ).expect("create spinner");
    H_SPIN.store(spin.0 as isize, Ordering::SeqCst);
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

/// 菊花子窗口过程：WM_TIMER 推进角度并重绘，WM_PAINT 用 GDI Arc 画 3/4 圆弧。
/// 依据：微软文档 Arc(hdc, x1,y1,x2,y2, x3,y3,x4,y4) —— 边界矩形 + 弧起点/终点射线，
/// 起止点取圆周上按当前角度计算的坐标。
unsafe extern "system" fn spinner_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // 注意：match arm 必须用【已导入的常量】（WM_TIMER/WM_PAINT），否则裸标识符是
    // 变量绑定模式（匹配一切值、抢在 _ 之前），曾导致 WM_NCCREATE 返回 0 使窗口创建失败。
    match msg {
        WM_TIMER => {
            SPIN_ANGLE.fetch_add(SPIN_STEP_DEG, Ordering::SeqCst);
            let _ = InvalidateRect(Some(hwnd), None, true);
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rc = RECT::default();
            let _ = GetClientRect(hwnd, &mut rc);
            let cx = (rc.left + rc.right) as f32 / 2.0;
            let cy = (rc.top + rc.bottom) as f32 / 2.0;
            let r = ((rc.right - rc.left).min(rc.bottom - rc.top)) as f32 / 2.0 - 4.0;
            let a0 = SPIN_ANGLE.load(Ordering::SeqCst) as f32;
            let (sin0, cos0) = a0.to_radians().sin_cos();
            let (sin1, cos1) = (a0 + 270.0).to_radians().sin_cos();
            let pen = CreatePen(PS_SOLID, 3, COLORREF(0x00B49646)); // COLORREF 是 0x00BBGGRR → SteelBlue(70,130,180)
            let old = SelectObject(hdc, pen.into());
            Arc(hdc, rc.left, rc.top, rc.right, rc.bottom,
                (cx + r * cos0) as i32, (cy + r * sin0) as i32,
                (cx + r * cos1) as i32, (cy + r * sin1) as i32);
            SelectObject(hdc, old);
            let _ = DeleteObject(pen.into());
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// 进入/退出忙状态：禁用浏览/生成/重命名按钮，主窗口中央显示旋转菊花。
/// 与 C# 版行为一致：后台执行耗时逻辑，UI 线程消息循环保持畅通，窗口不会无响应。
unsafe fn set_busy(busy: bool) {
    BUSY.store(busy, Ordering::SeqCst);
    for atom in [&H_BTN_BROWSE, &H_BTN_GENERATE, &H_BTN_BROWSE_LIST, &H_BTN_RENAME] {
        EnableWindow(hwnd_of(atom), !busy);
    }
    let spin = hwnd_of(&H_SPIN);
    if busy {
        let mut rc = RECT::default();
        let _ = GetClientRect(hwnd_of(&H_MAIN), &mut rc);
        let w = sc(SPIN_SIZE);
        let h_ = sc(SPIN_SIZE);
        let x = (rc.right - rc.left - w) / 2;
        let y = (rc.bottom - rc.top - h_) / 2;
        // HWND_TOP 置于子控件最上层，SWP_SHOWWINDOW 同时显示
        SetWindowPos(spin, Some(HWND_TOP), x, y, w, h_, SWP_SHOWWINDOW);
        SetTimer(Some(spin), SPIN_TIMER_ID, SPIN_INTERVAL_MS, None);
    } else {
        KillTimer(Some(spin), SPIN_TIMER_ID);
        let _ = ShowWindow(spin, SW_HIDE);
    }
}

/// 把耗时任务放到 std::thread 执行，完成后 PostMessageW 把 Box<WorkResult>
/// 经 lParam 传回 UI 线程弹窗（Win32 跨线程回 UI 的标准做法）。
fn spawn_work(f: impl FnOnce() -> WorkResult + Send + 'static) {
    // HWND 含裸指针不实现 Send，传 isize 值、线程内再还原
    let hwnd_val = hwnd_of(&H_MAIN).0 as isize;
    std::thread::spawn(move || {
        let result = f();
        unsafe {
            let _ = PostMessageW(
                Some(HWND(hwnd_val as *mut core::ffi::c_void)),
                WM_WORKDONE,
                WPARAM(0),
                LPARAM(Box::into_raw(Box::new(result)) as isize),
            );
        }
    });
}

/// 展示重命名结果弹窗（与 C# 版文案一致）。
fn show_rename_result(r: logic::RenameResult) {
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
        // 忙碌时客户区显示等待光标（与 C# 版 UseWaitCursor 行为一致）
        WM_SETCURSOR => {
            if BUSY.load(Ordering::SeqCst) && (lparam.0 as u32 & 0xffff) == HTCLIENT as u32 {
                let c = LoadCursorW(None, IDC_WAIT).unwrap();
                SetCursor(Some(c));
                LRESULT(1)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        // 后台任务完成：取回 Box<WorkResult>，退出忙状态并弹窗
        WM_WORKDONE => {
            let boxed = unsafe { Box::from_raw(lparam.0 as *mut WorkResult) };
            unsafe { set_busy(false); }
            match *boxed {
                WorkResult::Generate(res, list_display) => match res {
                    Ok(n) => msg_box(
                        &format!("已生成清单：{}\n共 {} 个文件。\n填好第二列后，可在功能二中随时处理。", list_display, n),
                        "完成", MB_OK | MB_ICONINFORMATION),
                    Err(e) => msg_box(&format!("生成清单失败：\n{}", e), "错误", MB_OK | MB_ICONERROR),
                },
                WorkResult::Rename(res) => match res {
                    Ok(r) => show_rename_result(r),
                    Err(e) => msg_box(&format!("重命名失败：\n{}", e), "错误", MB_OK | MB_ICONERROR),
                },
            }
            LRESULT(0)
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
    // 扫描文件夹 + 写 xlsx 可能耗时：后台线程执行，期间显示菊花、禁用操作按钮
    unsafe { set_busy(true); }
    let folder = folder.to_string();
    spawn_work(move || {
        let list = std::path::Path::new(&folder).join(logic::LIST_FILE_NAME);
        let list_display = list.display().to_string();
        let res = logic::generate_list(std::path::Path::new(&folder), &filter)
            .map(|names| names.len())
            .map_err(|e| e.to_string());
        WorkResult::Generate(res, list_display)
    });
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
    // 读取清单 + 哈希 + 改名可能耗时：后台线程执行，期间显示菊花、禁用操作按钮
    unsafe { set_busy(true); }
    let list = list.to_path_buf();
    spawn_work(move || {
        let res = logic::rename(&list, &template).map_err(|e| e.to_string());
        WorkResult::Rename(res)
    });
}
