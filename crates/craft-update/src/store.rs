//! Copies installed by an app store (Microsoft Store, Mac App Store) are updated by that store, so
//! they never download or replace themselves. Store policies forbid it, and the store would fight
//! the change.

use std::path::Path;

/// Whether this running program was installed by an app store.
pub fn installed() -> bool {
    std::env::current_exe().map(|exe| installed_at(&exe)).unwrap_or(false)
}

/// Whether a program at `exe` sits where a store installs it: Windows keeps MSIX packages under a
/// `WindowsApps` folder, and a Mac App Store app carries a `Contents/_MASReceipt` folder.
pub fn installed_at(exe: &Path) -> bool {
    let windows_apps = exe.components().any(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case("WindowsApps"));
    let mac_receipt = exe.ancestors().any(|dir| dir.file_name().is_some_and(|n| n == "Contents") && dir.join("_MASReceipt").is_dir());
    windows_apps || mac_receipt
}

/// The store's own updates page, opened by Help ▸ Check for Updates in a store copy.
pub fn updates_url() -> &'static str {
    if cfg!(target_os = "macos") { "macappstore://showUpdatesPage" } else { "ms-windows-store://downloadsandupdates" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_under_windows_apps_is_a_store_copy() {
        let exe = Path::new("C:/Program Files/WindowsApps/NasserAlqahtani.Kalima_1.0.0.0_x64__abc/Kalima.exe");
        assert!(installed_at(exe));
        assert!(installed_at(Path::new("C:/Program Files/windowsapps/X/Kalima.exe")));
    }

    #[test]
    fn a_downloaded_program_is_not_a_store_copy() {
        assert!(!installed_at(Path::new("C:/Users/n/Downloads/Kalima/Kalima.exe")));
        assert!(!installed_at(Path::new("/Applications/كلمة.app/Contents/MacOS/wordcraft")));
        assert!(!installed_at(Path::new("C:/MyWindowsAppsBackup/Kalima.exe")));
    }

    #[test]
    fn a_mac_app_with_a_store_receipt_is_a_store_copy() {
        let dir = std::env::temp_dir().join(format!("kalima-store-test-{}", std::process::id()));
        let contents = dir.join("Kalima.app").join("Contents");
        std::fs::create_dir_all(contents.join("_MASReceipt")).unwrap();
        std::fs::create_dir_all(contents.join("MacOS")).unwrap();
        assert!(installed_at(&contents.join("MacOS").join("wordcraft")));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
