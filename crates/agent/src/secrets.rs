//! API keys. On macOS they live in the Keychain under the service «Mokteb AI», so one key works in
//! every مكتب app. Elsewhere (and if the Keychain refuses) they live in `agent-keys.json` in the
//! app's settings folder, readable only by this user.

use std::collections::BTreeMap;
use std::path::Path;

use crate::Result;
use crate::settings::write_atomic;

#[cfg(target_os = "macos")]
const SERVICE: &str = "Mokteb AI";
const FILE: &str = "agent-keys.json";

fn read_file(dir: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(dir.join(FILE)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn write_file(dir: &Path, keys: &BTreeMap<String, String>) -> Result<()> {
    let f = dir.join(FILE);
    write_atomic(&f, serde_json::to_string_pretty(keys)?.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// The key saved for `provider`, or an empty string.
pub fn get(dir: &Path, provider: &str) -> String {
    #[cfg(target_os = "macos")]
    if let Ok(k) = security_framework::passwords::get_generic_password(SERVICE, provider)
        && let Ok(k) = String::from_utf8(k)
    {
        return k;
    }
    read_file(dir).get(provider).cloned().unwrap_or_default()
}

/// Save `key` for `provider`; an empty key removes it.
pub fn set(dir: &Path, provider: &str, key: &str) -> Result<()> {
    let key = key.trim();
    #[cfg(target_os = "macos")]
    {
        let stored = if key.is_empty() {
            // Nothing saved is fine too.
            let _ = security_framework::passwords::delete_generic_password(SERVICE, provider);
            true
        } else {
            security_framework::passwords::set_generic_password(SERVICE, provider, key.as_bytes()).is_ok()
        };
        if stored {
            // Remove any copy left in the file from before.
            let mut keys = read_file(dir);
            if keys.remove(provider).is_some() {
                write_file(dir, &keys)?;
            }
            return Ok(());
        }
    }
    let mut keys = read_file(dir);
    if key.is_empty() {
        keys.remove(provider);
    } else {
        keys.insert(provider.to_string(), key.to_string());
    }
    write_file(dir, &keys)
}

#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    use super::*;

    #[test]
    fn keys_are_saved_and_removed() {
        let dir = std::env::temp_dir().join(format!("kalima-agent-keys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(get(&dir, "gemini"), "");
        set(&dir, "gemini", "  abc  ").unwrap();
        assert_eq!(get(&dir, "gemini"), "abc");
        set(&dir, "gemini", "").unwrap();
        assert_eq!(get(&dir, "gemini"), "");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
