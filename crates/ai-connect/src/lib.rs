//! Connect the app's MCP server to the AI apps on this computer with one button, instead of
//! asking people to edit settings files by hand.
//!
//! Each AI app keeps its MCP servers in its own settings file. [`connect`] adds the app's server
//! there (and [`disconnect`] takes it out again), leaving everything else in the file as it was:
//! - Claude (the desktop app), Claude Code, Cursor, Windsurf, Gemini CLI and LM Studio: a JSON
//!   file with an `mcpServers` object;
//! - VS Code: a JSON `mcp.json` with a `servers` object; when that file has comments (which
//!   VS Code allows), it is left alone and VS Code's own install link is returned instead;
//! - Codex: a `[mcp_servers.<name>]` table in `~/.codex/config.toml`.
//!
//! A file that can't be read as JSON is never rewritten. The first time a file is changed, the
//! original is kept beside it (`<file>.<name>-backup`), and every write goes through a temporary
//! file and a rename, so a crash never leaves half a settings file.
//!
//! Nothing here depends on the user interface; the app's window calls these functions.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

/// The MCP server to connect: its name in the AI apps' settings, the program and its arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Server {
    /// The key in the AI apps' settings, e.g. `kalima`. ASCII letters, digits, `-` and `_`.
    pub key: String,
    /// The absolute path of the MCP server program.
    pub command: PathBuf,
    /// Its arguments, e.g. `["mcp"]`.
    pub args: Vec<String>,
}

impl Server {
    fn command_str(&self) -> String {
        self.command.to_string_lossy().into_owned()
    }

    /// The standard `mcpServers` entry, which most AI apps accept as is.
    pub fn entry(&self) -> Value {
        json!({ "command": self.command_str(), "args": self.args })
    }

    /// A ready-to-paste `{"mcpServers": {...}}` block for any other MCP app.
    pub fn snippet(&self) -> String {
        let mut servers = Map::new();
        servers.insert(self.key.clone(), self.entry());
        let v = json!({ "mcpServers": servers });
        serde_json::to_string_pretty(&v).unwrap_or_default()
    }
}

/// The folders AI apps keep their settings in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dirs {
    /// The user's home folder.
    pub home: PathBuf,
    /// The per-user application settings folder: macOS `~/Library/Application Support`, Windows
    /// `%APPDATA%`, otherwise `$XDG_CONFIG_HOME` or `~/.config`.
    pub config: PathBuf,
    /// Windows `%LOCALAPPDATA%` (where some apps are installed); `None` elsewhere.
    pub local: Option<PathBuf>,
    /// Whether this is macOS (Claude is also found in `/Applications`).
    pub macos: bool,
}

impl Dirs {
    /// This computer's folders, or `None` when the home folder is unknown.
    pub fn system() -> Option<Self> {
        let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
        let home = if cfg!(windows) { var("USERPROFILE").or_else(|| var("HOME")) } else { var("HOME") }?;
        let (config, local) = if cfg!(target_os = "macos") {
            (home.join("Library").join("Application Support"), None)
        } else if cfg!(windows) {
            let roaming = var("APPDATA").unwrap_or_else(|| home.join("AppData").join("Roaming"));
            let local = var("LOCALAPPDATA").unwrap_or_else(|| home.join("AppData").join("Local"));
            (roaming, Some(local))
        } else {
            (var("XDG_CONFIG_HOME").unwrap_or_else(|| home.join(".config")), None)
        };
        Some(Self { home, config, local, macos: cfg!(target_os = "macos") })
    }
}

/// An AI app that can use MCP servers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Client {
    Claude,
    ClaudeCode,
    Codex,
    Cursor,
    VsCode,
    Windsurf,
    Gemini,
    LmStudio,
}

/// How the AI app keeps its servers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Format {
    /// JSON, servers under `key`, each entry with `"type": "stdio"` when `typed`.
    Json { key: &'static str, typed: bool },
    /// Codex's TOML.
    CodexToml,
}

impl Client {
    /// Every supported AI app, in the order the window lists them.
    pub const ALL: [Client; 8] =
        [Client::Claude, Client::ClaudeCode, Client::Codex, Client::Cursor, Client::VsCode, Client::Windsurf, Client::Gemini, Client::LmStudio];

    /// The app's own name (not translated).
    pub fn name(self) -> &'static str {
        match self {
            Client::Claude => "Claude",
            Client::ClaudeCode => "Claude Code",
            Client::Codex => "Codex",
            Client::Cursor => "Cursor",
            Client::VsCode => "VS Code",
            Client::Windsurf => "Windsurf",
            Client::Gemini => "Gemini CLI",
            Client::LmStudio => "LM Studio",
        }
    }

    /// A stable id for commands and settings: `claude`, `claudeCode`, `codex`, ...
    pub fn id(self) -> &'static str {
        match self {
            Client::Claude => "claude",
            Client::ClaudeCode => "claudeCode",
            Client::Codex => "codex",
            Client::Cursor => "cursor",
            Client::VsCode => "vscode",
            Client::Windsurf => "windsurf",
            Client::Gemini => "gemini",
            Client::LmStudio => "lmstudio",
        }
    }

    /// The client with this [`id`](Self::id).
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.id() == id)
    }

    /// Whether the app has to be restarted (or its window reloaded) to see a change.
    pub fn needs_restart(self) -> bool {
        matches!(self, Client::Claude | Client::ClaudeCode | Client::Codex | Client::Gemini | Client::LmStudio)
    }

    fn format(self) -> Format {
        match self {
            Client::VsCode => Format::Json { key: "servers", typed: true },
            Client::ClaudeCode => Format::Json { key: "mcpServers", typed: true },
            Client::Codex => Format::CodexToml,
            _ => Format::Json { key: "mcpServers", typed: false },
        }
    }

    /// The settings file the app keeps its MCP servers in.
    pub fn file(self, dirs: &Dirs) -> PathBuf {
        let h = &dirs.home;
        match self {
            Client::Claude => dirs.config.join("Claude").join("claude_desktop_config.json"),
            Client::ClaudeCode => h.join(".claude.json"),
            Client::Codex => h.join(".codex").join("config.toml"),
            Client::Cursor => h.join(".cursor").join("mcp.json"),
            Client::VsCode => dirs.config.join("Code").join("User").join("mcp.json"),
            Client::Windsurf => h.join(".codeium").join("windsurf").join("mcp_config.json"),
            Client::Gemini => h.join(".gemini").join("settings.json"),
            Client::LmStudio => h.join(".lmstudio").join("mcp.json"),
        }
    }

    /// Whether the app seems to be installed (its settings folder exists).
    pub fn installed(self, dirs: &Dirs) -> bool {
        let h = &dirs.home;
        match self {
            Client::Claude => {
                dirs.config.join("Claude").is_dir()
                    || (dirs.macos && Path::new("/Applications/Claude.app").is_dir())
                    || dirs.local.as_ref().is_some_and(|l| l.join("AnthropicClaude").is_dir())
            }
            Client::ClaudeCode => h.join(".claude.json").is_file() || h.join(".claude").is_dir(),
            Client::Codex => h.join(".codex").is_dir(),
            Client::Cursor => h.join(".cursor").is_dir(),
            Client::VsCode => dirs.config.join("Code").is_dir(),
            Client::Windsurf => h.join(".codeium").join("windsurf").is_dir(),
            Client::Gemini => h.join(".gemini").is_dir(),
            Client::LmStudio => h.join(".lmstudio").is_dir(),
        }
    }
}

/// Where an AI app stands with the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// The app isn't installed.
    NotInstalled,
    /// Installed, without the server.
    NotConnected,
    /// The server is set up with this program.
    Connected,
    /// The server is set up with another program (the app moved): connecting again updates it.
    Outdated,
    /// The settings file couldn't be read; it is left alone.
    Unreadable(String),
}

/// What [`connect`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Done {
    /// The settings were saved.
    Saved,
    /// The settings file can't be changed safely; open this link and the AI app adds the server
    /// itself (VS Code with a commented `mcp.json`).
    OpenLink(String),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not read {path}: {source}")]
    Read { path: PathBuf, source: std::io::Error },
    #[error("could not write {path}: {source}")]
    Write { path: PathBuf, source: std::io::Error },
    #[error("{path} is not valid JSON, so it was left unchanged: {detail}")]
    Parse { path: PathBuf, detail: String },
    #[error("the server name {0:?} is not allowed")]
    BadKey(String),
}

/// Settings files bigger than this are not read (a settings file is a few kilobytes; Claude
/// Code's history file can grow, but stays far below this).
const MAX_FILE: u64 = 64 * 1024 * 1024;

fn valid_key(key: &str) -> bool {
    !key.is_empty() && key.len() <= 64 && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Where `client` stands with `server`.
pub fn state(client: Client, dirs: &Dirs, server: &Server) -> State {
    if !client.installed(dirs) {
        return State::NotInstalled;
    }
    let path = client.file(dirs);
    let text = match read(&path) {
        Ok(Some(t)) => t,
        Ok(None) => return State::NotConnected,
        Err(e) => return State::Unreadable(e.to_string()),
    };
    match client.format() {
        Format::Json { key, typed } => {
            let Ok(root) = serde_json::from_str::<Value>(&text) else {
                // VS Code's file may have comments: connecting then uses its install link.
                return if client == Client::VsCode {
                    State::NotConnected
                } else {
                    State::Unreadable(format!("{} is not valid JSON", path.display()))
                };
            };
            match root.get(key).and_then(|s| s.get(&server.key)) {
                None => State::NotConnected,
                Some(e) if *e == json_entry(server, typed) || same_program(e, server) => State::Connected,
                Some(_) => State::Outdated,
            }
        }
        Format::CodexToml => match toml_section(&text, &server.key) {
            None => State::NotConnected,
            Some((start, end)) => {
                let want = format!("command = {}", toml_str(&server.command_str()));
                let lines: Vec<&str> = text.lines().collect();
                let has = lines.get(start..end).is_some_and(|ls| ls.iter().any(|l| l.trim() == want));
                if has { State::Connected } else { State::Outdated }
            }
        },
    }
}

fn same_program(entry: &Value, server: &Server) -> bool {
    let command = entry.get("command").and_then(Value::as_str);
    let args: Option<Vec<&str>> = entry.get("args").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect());
    command == Some(server.command_str().as_str()) && args.is_some_and(|a| a == server.args.iter().map(String::as_str).collect::<Vec<_>>())
}

fn json_entry(server: &Server, typed: bool) -> Value {
    if typed { json!({ "type": "stdio", "command": server.command_str(), "args": server.args }) } else { server.entry() }
}

/// Add `server` to `client`'s settings (or bring an outdated entry up to date).
pub fn connect(client: Client, dirs: &Dirs, server: &Server) -> Result<Done, Error> {
    if !valid_key(&server.key) {
        return Err(Error::BadKey(server.key.clone()));
    }
    let path = client.file(dirs);
    let text = read(&path)?;
    match client.format() {
        Format::Json { key, typed } => {
            let mut root = match text.as_deref().map(str::trim) {
                None | Some("") => Value::Object(Map::new()),
                Some(t) => match serde_json::from_str::<Value>(t) {
                    Ok(v) if v.is_object() => v,
                    Ok(_) => return Err(Error::Parse { path, detail: "the file is not a JSON object".into() }),
                    Err(_) if client == Client::VsCode => return Ok(Done::OpenLink(vscode_link(server))),
                    Err(e) => return Err(Error::Parse { path, detail: e.to_string() }),
                },
            };
            let Some(obj) = root.as_object_mut() else {
                return Err(Error::Parse { path, detail: "the file is not a JSON object".into() });
            };
            let servers = obj.entry(key).or_insert_with(|| Value::Object(Map::new()));
            if !servers.is_object() {
                return Err(Error::Parse { path, detail: format!("\"{key}\" is not a JSON object") });
            }
            if let Some(s) = servers.as_object_mut() {
                s.insert(server.key.clone(), json_entry(server, typed));
            }
            let out = serde_json::to_string_pretty(&root).map_err(|e| Error::Parse { path: path.clone(), detail: e.to_string() })?;
            write(&path, text.is_some(), &server.key, &(out + "\n"))?;
        }
        Format::CodexToml => {
            let old = text.clone().unwrap_or_default();
            let base = match toml_section(&old, &server.key) {
                Some((start, end)) => remove_lines(&old, start, end),
                None => old,
            };
            let mut out = base.trim_end().to_string();
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            let args: Vec<String> = server.args.iter().map(|a| toml_str(a)).collect();
            let _ = write!(out, "[mcp_servers.{}]\ncommand = {}\nargs = [{}]\n", server.key, toml_str(&server.command_str()), args.join(", "));
            write(&path, text.is_some(), &server.key, &out)?;
        }
    }
    Ok(Done::Saved)
}

/// Take `server` out of `client`'s settings. Nothing happens when it isn't there.
pub fn disconnect(client: Client, dirs: &Dirs, server: &Server) -> Result<(), Error> {
    let path = client.file(dirs);
    let Some(text) = read(&path)? else { return Ok(()) };
    match client.format() {
        Format::Json { key, .. } => {
            let mut root: Value = serde_json::from_str(&text).map_err(|e| Error::Parse { path: path.clone(), detail: e.to_string() })?;
            let removed = root.get_mut(key).and_then(Value::as_object_mut).and_then(|s| s.remove(&server.key)).is_some();
            if removed {
                let out = serde_json::to_string_pretty(&root).map_err(|e| Error::Parse { path: path.clone(), detail: e.to_string() })?;
                write(&path, true, &server.key, &(out + "\n"))?;
            }
        }
        Format::CodexToml => {
            if let Some((start, end)) = toml_section(&text, &server.key) {
                let out = remove_lines(&text, start, end);
                let out = if out.trim().is_empty() { String::new() } else { format!("{}\n", out.trim_end()) };
                write(&path, true, &server.key, &out)?;
            }
        }
    }
    Ok(())
}

/// VS Code's install link: VS Code opens and asks the user to add the server.
pub fn vscode_link(server: &Server) -> String {
    let config = json!({ "name": server.key, "command": server.command_str(), "args": server.args });
    format!("vscode:mcp/install?{}", percent_encode(&config.to_string()))
}

/// The MCP server program beside the running app: the first of `names` (without `.exe`) found in
/// the app's folder.
pub fn find_server(exe: &Path, names: &[&str]) -> Option<PathBuf> {
    let dir = exe.parent()?;
    names.iter().map(|n| dir.join(format!("{n}{}", std::env::consts::EXE_SUFFIX))).find(|p| p.is_file())
}

/// Whether the app runs from a place it won't stay in: a mounted disk image (`/Volumes/...`) or a
/// macOS quarantine copy (App Translocation). Settings pointing there break once it is closed.
pub fn temporary_location(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains("/AppTranslocation/") || (s.starts_with("/Volumes/") && cfg!(target_os = "macos"))
}

fn read(path: &Path) -> Result<Option<String>, Error> {
    match std::fs::metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(Error::Read { path: path.into(), source }),
        Ok(m) if m.len() > MAX_FILE => {
            return Err(Error::Read { path: path.into(), source: std::io::Error::other("the file is too large") });
        }
        Ok(_) => {}
    }
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).trim_start_matches('\u{feff}').to_string())),
        Err(source) => Err(Error::Read { path: path.into(), source }),
    }
}

/// Write `text` to `path` through a temporary file, keeping the original once beside it.
fn write(path: &Path, existed: bool, key: &str, text: &str) -> Result<(), Error> {
    let err = |source| Error::Write { path: path.into(), source };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if existed {
        let backup = path.with_file_name(format!("{name}.{key}-backup"));
        if !backup.exists() {
            std::fs::copy(path, &backup).map_err(err)?;
        }
    }
    let tmp = path.with_file_name(format!(".{name}.{key}-{}.tmp", std::process::id()));
    std::fs::write(&tmp, text).map_err(err)?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        err(e)
    })
}

/// The lines `[start, end)` of `[mcp_servers.<key>]` (and its sub-tables such as
/// `[mcp_servers.<key>.env]`) in Codex's TOML.
fn toml_section(text: &str, key: &str) -> Option<(usize, usize)> {
    let headers = [format!("[mcp_servers.{key}]"), format!("[mcp_servers.\"{key}\"]")];
    let subs = [format!("[mcp_servers.{key}."), format!("[mcp_servers.\"{key}\".")];
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|l| headers.iter().any(|h| l.trim() == h))?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, l)| {
            let t = l.trim();
            t.starts_with('[') && !subs.iter().any(|s| t.starts_with(s.as_str()))
        })
        .map_or(lines.len(), |(i, _)| i);
    Some((start, end))
}

fn remove_lines(text: &str, start: usize, end: usize) -> String {
    let mut out = String::new();
    for (i, l) in text.lines().enumerate() {
        if i < start || i >= end {
            out.push_str(l);
            out.push('\n');
        }
    }
    out
}

/// A TOML basic string.
fn toml_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests;
