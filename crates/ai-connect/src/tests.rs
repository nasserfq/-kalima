use super::*;

fn setup() -> (tempfile::TempDir, Dirs, Server) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let dirs = Dirs { config: home.join("config"), home, local: None, macos: false };
    std::fs::create_dir_all(&dirs.config).unwrap();
    let server = Server { key: "kalima".into(), command: PathBuf::from(r"C:\Apps\كلمة\kalima-cli.exe"), args: vec!["mcp".into()] };
    (tmp, dirs, server)
}

fn install(client: Client, dirs: &Dirs) {
    let file = client.file(dirs);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    if client == Client::ClaudeCode {
        std::fs::create_dir_all(dirs.home.join(".claude")).unwrap();
    }
}

#[test]
fn every_client_connects_and_disconnects() {
    let (_tmp, dirs, server) = setup();
    for client in Client::ALL {
        assert_eq!(state(client, &dirs, &server), State::NotInstalled, "{client:?}");
        install(client, &dirs);
        assert_eq!(state(client, &dirs, &server), State::NotConnected, "{client:?}");
        assert_eq!(connect(client, &dirs, &server).unwrap(), Done::Saved, "{client:?}");
        assert_eq!(state(client, &dirs, &server), State::Connected, "{client:?}");
        disconnect(client, &dirs, &server).unwrap();
        assert_eq!(state(client, &dirs, &server), State::NotConnected, "{client:?}");
    }
}

#[test]
fn other_settings_are_kept_and_backed_up() {
    let (_tmp, dirs, server) = setup();
    install(Client::Claude, &dirs);
    let file = Client::Claude.file(&dirs);
    let original = r#"{"mcpServers":{"other":{"command":"x"}},"globalShortcut":"Ctrl+Space"}"#;
    std::fs::write(&file, original).unwrap();
    connect(Client::Claude, &dirs, &server).unwrap();
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(v["globalShortcut"], "Ctrl+Space");
    assert_eq!(v["mcpServers"]["other"]["command"], "x");
    assert_eq!(v["mcpServers"]["kalima"]["command"], r"C:\Apps\كلمة\kalima-cli.exe");
    assert_eq!(v["mcpServers"]["kalima"]["args"], json!(["mcp"]));
    let backup = file.with_file_name("claude_desktop_config.json.kalima-backup");
    assert_eq!(std::fs::read_to_string(backup).unwrap(), original);
}

#[test]
fn broken_json_is_never_rewritten() {
    let (_tmp, dirs, server) = setup();
    install(Client::Cursor, &dirs);
    let file = Client::Cursor.file(&dirs);
    std::fs::write(&file, "{ // my servers\n }").unwrap();
    assert!(matches!(state(Client::Cursor, &dirs, &server), State::Unreadable(_)));
    assert!(matches!(connect(Client::Cursor, &dirs, &server), Err(Error::Parse { .. })));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "{ // my servers\n }");
    std::fs::write(&file, "[1, 2]").unwrap();
    assert!(connect(Client::Cursor, &dirs, &server).is_err());
    std::fs::write(&file, r#"{"mcpServers": 3}"#).unwrap();
    assert!(connect(Client::Cursor, &dirs, &server).is_err());
}

#[test]
fn vscode_with_comments_gets_its_install_link() {
    let (_tmp, dirs, server) = setup();
    install(Client::VsCode, &dirs);
    let file = Client::VsCode.file(&dirs);
    std::fs::write(&file, "{\n // mine\n \"servers\": {}\n}").unwrap();
    let Done::OpenLink(link) = connect(Client::VsCode, &dirs, &server).unwrap() else { panic!("expected a link") };
    assert!(link.starts_with("vscode:mcp/install?%7B"), "{link}");
    assert!(link.contains("%22name%22%3A%22kalima%22") && !link.contains(' '), "{link}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "{\n // mine\n \"servers\": {}\n}");
}

#[test]
fn vscode_and_claude_code_entries_are_typed() {
    let (_tmp, dirs, server) = setup();
    for client in [Client::VsCode, Client::ClaudeCode] {
        install(client, &dirs);
        connect(client, &dirs, &server).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(client.file(&dirs)).unwrap()).unwrap();
        let key = if client == Client::VsCode { "servers" } else { "mcpServers" };
        assert_eq!(v[key]["kalima"]["type"], "stdio");
    }
}

#[test]
fn moved_app_shows_outdated_and_reconnecting_updates_it() {
    let (_tmp, dirs, server) = setup();
    let moved = Server { command: PathBuf::from("/Applications/كلمة.app/Contents/MacOS/kalima-cli"), ..server.clone() };
    for client in [Client::Gemini, Client::Codex] {
        install(client, &dirs);
        connect(client, &dirs, &server).unwrap();
        assert_eq!(state(client, &dirs, &moved), State::Outdated, "{client:?}");
        connect(client, &dirs, &moved).unwrap();
        assert_eq!(state(client, &dirs, &moved), State::Connected, "{client:?}");
    }
    let toml = std::fs::read_to_string(Client::Codex.file(&dirs)).unwrap();
    assert_eq!(toml.matches("[mcp_servers.kalima]").count(), 1, "{toml}");
}

#[test]
fn codex_toml_keeps_other_tables() {
    let (_tmp, dirs, server) = setup();
    install(Client::Codex, &dirs);
    let file = Client::Codex.file(&dirs);
    let original =
        "model = \"o3\"\n\n[mcp_servers.kalima]\ncommand = \"old\"\n\n[mcp_servers.kalima.env]\nA = \"1\"\n\n[mcp_servers.other]\ncommand = \"y\"\n";
    std::fs::write(&file, original).unwrap();
    connect(Client::Codex, &dirs, &server).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.starts_with("model = \"o3\"\n"), "{text}");
    assert!(text.contains("[mcp_servers.other]\ncommand = \"y\""), "{text}");
    assert!(!text.contains("old") && !text.contains("A = \"1\""), "{text}");
    assert!(text.contains(r#"command = "C:\\Apps\\كلمة\\kalima-cli.exe""#), "{text}");
    assert!(text.contains("args = [\"mcp\"]"), "{text}");
    disconnect(Client::Codex, &dirs, &server).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert_eq!(text, "model = \"o3\"\n\n[mcp_servers.other]\ncommand = \"y\"\n");
}

#[test]
fn hostile_keys_and_text_are_refused_or_escaped() {
    let (_tmp, dirs, server) = setup();
    install(Client::Codex, &dirs);
    for key in ["", "a]\n[x", "a.b", "كلمة"] {
        let bad = Server { key: key.into(), ..server.clone() };
        assert!(matches!(connect(Client::Codex, &dirs, &bad), Err(Error::BadKey(_))), "{key:?}");
    }
    assert_eq!(toml_str("a\"b\\c\n\u{1}"), r#""a\"b\\c\n\u0001""#);
    assert_eq!(percent_encode("a b/ك"), "a%20b%2F%D9%83");
}

#[test]
fn snippet_is_ready_to_paste() {
    let (_tmp, _dirs, server) = setup();
    let v: Value = serde_json::from_str(&server.snippet()).unwrap();
    assert_eq!(v["mcpServers"]["kalima"]["args"], json!(["mcp"]));
}

#[test]
fn client_ids_round_trip() {
    for c in Client::ALL {
        assert_eq!(Client::from_id(c.id()), Some(c));
    }
    assert_eq!(Client::from_id("nope"), None);
}
