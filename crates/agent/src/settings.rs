//! The agent's settings (which service and model, custom services), saved as `agent.json` in the
//! app's settings folder.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::providers::{Provider, builtin_providers};

const FILE: &str = "agent.json";

/// Write `bytes` to `path` through a temporary file, so a crash never leaves half a file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The service the agent uses (a provider id); empty until the person picks one.
    pub provider: String,
    /// The model on that service; empty means the service's default.
    pub model: String,
    /// "low", "medium" or "high": how hard Claude thinks.
    pub effort: String,
    /// The OpenAI-compatible services the person added (the built-in ones are always there).
    pub custom_providers: Vec<Provider>,
}

impl Settings {
    /// The saved settings, or the defaults when there are none or the file can't be read.
    pub fn load(dir: &Path) -> Self {
        std::fs::read_to_string(dir.join(FILE)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        write_atomic(&dir.join(FILE), serde_json::to_string_pretty(self)?.as_bytes())
    }

    /// Every service: the built-in ones, then the person's own.
    pub fn providers(&self) -> Vec<Provider> {
        let mut v = builtin_providers();
        v.extend(self.custom_providers.iter().cloned().map(|mut p| {
            p.builtin = false;
            p
        }));
        v
    }

    pub fn provider(&self, id: &str) -> Option<Provider> {
        self.providers().into_iter().find(|p| p.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_survive_junk() {
        let dir = std::env::temp_dir().join(format!("kalima-agent-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(Settings::load(&dir), Settings::default(), "no file yet");
        let s = Settings { provider: "gemini".into(), model: "gemini-flash-latest".into(), ..Default::default() };
        s.save(&dir).unwrap();
        assert_eq!(Settings::load(&dir), s);
        std::fs::write(dir.join(FILE), "{not json").unwrap();
        assert_eq!(Settings::load(&dir), Settings::default(), "a broken file reads as the defaults");
        assert!(s.provider("anthropic").is_some() && s.provider("nope").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
