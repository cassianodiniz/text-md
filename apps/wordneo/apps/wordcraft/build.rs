//! Windows only: embed the app icon and version info (VERSIONINFO) into `wordcraft.exe`, so it
//! shows in Explorer, the taskbar, the Start menu and Alt-Tab.
//!
//! On every other target this does nothing. A missing resource compiler is a warning, so a
//! cross-compile from macOS or Linux still links, unless `WORDCRAFT_REQUIRE_WINRES=1` turns it
//! into an error (for release builds).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/app-icon/wordcraft.ico");
    println!("cargo:rerun-if-env-changed=WORDCRAFT_REQUIRE_WINRES");
    println!("cargo:rerun-if-env-changed=WORDNEO_NATIVE_LIB_DIR");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    // Portable GNU toolchains may omit the import library used by webbrowser.
    // This is a build input only; the DLL remains the Windows system library.
    if let Some(path) = std::env::var_os("WORDNEO_NATIVE_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", std::path::Path::new(&path).display());
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/app-icon/wordcraft.ico")
        .set("ProductName", "WordNeo")
        .set("FileDescription", "WordNeo word processor, based on WordCraft")
        .set("CompanyName", "Learning Machines LLC")
        .set("LegalCopyright", "Copyright (c) the WordCraft authors. MIT OR Apache-2.0.")
        .set("OriginalFilename", "WordNeo.exe")
        .set("InternalName", "wordneo");
    if let Err(e) = res.compile() {
        if std::env::var_os("WORDCRAFT_REQUIRE_WINRES").is_some() {
            eprintln!("embedding Windows resources failed: {e}");
            std::process::exit(1);
        }
        println!("cargo:warning=wordcraft.exe built without icon/version resources: {e}");
    }
}
