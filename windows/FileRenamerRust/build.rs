// 用 Windows SDK 的 rc.exe 把 app.manifest 编译成资源并链接进 exe。
// 依据：微软文档 "Enabling Visual Styles" —— 必须声明 ComCtl32 v6 依赖，
// 否则标准控件使用 v5 的经典外观（灰色立体按钮/STATIC 灰底、无 ClearType）。
use std::env;
use std::path::PathBuf;
use std::process::Command;

fn find_rc_exe() -> Option<PathBuf> {
    // 1) 环境变量显式指定
    if let Ok(p) = env::var("RC_EXE") {
        let p = PathBuf::from(p);
        if p.is_file() { return Some(p); }
    }
    // 2) 扫描 Windows Kits 10 bin 下所有版本的 x64/rc.exe，取最新
    let base = PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let mut found: Vec<(String, PathBuf)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&base) {
        for e in entries.flatten() {
            let rc = e.path().join("x64").join("rc.exe");
            if rc.is_file() {
                found.push((e.file_name().to_string_lossy().into_owned(), rc));
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found.into_iter().next_back().map(|(_, p)| p)
}

fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=app.rc");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" { return; }

    let rc = find_rc_exe().expect("Windows SDK rc.exe not found; set RC_EXE");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let res = out.join("app.res");

    let status = Command::new(&rc)
        .args(["/nologo", "/fo"])
        .arg(&res)
        .arg("app.rc")
        .current_dir(env::var("CARGO_MANIFEST_DIR").unwrap())
        .status()
        .expect("failed to run rc.exe");
    assert!(status.success(), "rc.exe failed with {status}");

    println!("cargo:rustc-link-arg={}", res.display());
}
