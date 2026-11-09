//! Windows only: embed the app icon and version info (VERSIONINFO) into the app's executable
//! (shipped as `Kalima.exe`), so it shows in Explorer, the taskbar, the Start menu and Alt-Tab.
//!
//! On every other target this does nothing. A missing resource compiler is a warning, so a
//! cross-compile from macOS or Linux still links, unless `WORDCRAFT_REQUIRE_WINRES=1` turns it
//! into an error (for release builds).

#[path = "src/windows_version.rs"]
mod windows_version;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/windows_version.rs");
    println!("cargo:rerun-if-changed=../../assets/app-icon/wordcraft.ico");
    println!("cargo:rerun-if-env-changed=WORDCRAFT_REQUIRE_WINRES");
    println!("cargo:rerun-if-env-changed=KALIMA_VERSION");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let version = windows_version::version(env!("CARGO_PKG_VERSION"));
    let number = windows_version::version_number(&version);
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/app-icon/wordcraft.ico")
        .set("ProductName", windows_version::PRODUCT_NAME)
        .set("FileDescription", "كلمة: Arabic word processor")
        .set("CompanyName", windows_version::COMPANY_NAME)
        .set("LegalCopyright", windows_version::COPYRIGHT)
        .set("OriginalFilename", "Kalima.exe")
        .set("InternalName", "kalima")
        .set("FileVersion", &version)
        .set("ProductVersion", &version)
        .set_version_info(winresource::VersionInfo::FILEVERSION, number)
        .set_version_info(winresource::VersionInfo::PRODUCTVERSION, number);
    if let Err(e) = res.compile() {
        if std::env::var_os("WORDCRAFT_REQUIRE_WINRES").is_some() {
            eprintln!("embedding Windows resources failed: {e}");
            std::process::exit(1);
        }
        println!("cargo:warning=Kalima.exe built without icon/version resources: {e}");
    }
}
