//! Help ▸ Connect to AI («ربط مع الذكاء الاصطناعي»): connects كلمة's MCP server (`kalima-cli mcp`,
//! shipped beside the app) to the AI apps on this computer with one button each, by writing their
//! own settings (the `ai-connect` crate). Other MCP apps get the settings to paste.
//!
//! The window speaks Arabic in the Arabic interface and English otherwise. The web build has no
//! MCP server and no settings files, so the command reports that instead.

use crate::WordApp;

/// Strings: Arabic in the Arabic interface, English otherwise.
fn l(ar: &'static str, en: &'static str) -> &'static str {
    // English goes through the catalog like every other interface text.
    if crate::i18n::current().code() == "ar" { ar } else { crate::i18n::t(en) }
}

/// The window's title, also its place in File (Backstage).
pub(crate) fn title() -> &'static str {
    l("ربط مع الذكاء الاصطناعي", "Connect to AI Apps")
}

#[derive(Default)]
pub(crate) struct Runtime {
    pub(crate) open: bool,
    #[cfg(not(target_arch = "wasm32"))]
    rows: Vec<(ai_connect::Client, ai_connect::State)>,
    /// The last action's result: text and whether it went wrong.
    message: Option<(String, bool)>,
}

/// The MCP server beside the running app.
#[cfg(not(target_arch = "wasm32"))]
fn server() -> Option<ai_connect::Server> {
    let exe = std::env::current_exe().ok()?;
    let command = ai_connect::find_server(&exe, &["kalima-cli", "wordcraft-cli"])?;
    Some(ai_connect::Server { key: "kalima".into(), command, args: vec!["mcp".into()] })
}

#[cfg(not(target_arch = "wasm32"))]
fn refresh(rt: &mut Runtime) {
    rt.rows.clear();
    if let (Some(dirs), Some(server)) = (ai_connect::Dirs::system(), server()) {
        rt.rows = ai_connect::Client::ALL.into_iter().map(|c| (c, ai_connect::state(c, &dirs, &server))).collect();
    }
}

/// Open the window (the `ui.connectAi` command).
pub(crate) fn open(app: &mut WordApp) {
    app.connect_ai.open = true;
    app.connect_ai.message = None;
    #[cfg(not(target_arch = "wasm32"))]
    refresh(&mut app.connect_ai);
}

/// Connect or disconnect `client` without the window (`ui.connectAi` with `client` and
/// `disconnect`): what the control channel, tests and agents use.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn run(client: &str, disconnect: bool) -> Result<serde_json::Value, String> {
    let client = ai_connect::Client::from_id(client).ok_or_else(|| {
        let ids: Vec<&str> = ai_connect::Client::ALL.iter().map(|c| c.id()).collect();
        format!("unknown AI app {client:?}; one of: {}", ids.join(", "))
    })?;
    let dirs = ai_connect::Dirs::system().ok_or("the home folder is unknown")?;
    let server = server().ok_or("the MCP server (kalima-cli) is not beside the app")?;
    if disconnect {
        ai_connect::disconnect(client, &dirs, &server).map_err(|e| e.to_string())?;
        return Ok(serde_json::json!({ "client": client.id(), "state": "notConnected" }));
    }
    match ai_connect::connect(client, &dirs, &server).map_err(|e| e.to_string())? {
        ai_connect::Done::Saved => Ok(serde_json::json!({ "client": client.id(), "state": "connected" })),
        ai_connect::Done::OpenLink(link) => Ok(serde_json::json!({ "client": client.id(), "link": link })),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn state_label(s: &ai_connect::State) -> &'static str {
    use ai_connect::State;
    match s {
        State::NotInstalled => l("غير مثبت", "Not installed"),
        State::NotConnected => l("غير مربوط", "Not connected"),
        State::Connected => l("مربوط ✓", "Connected ✓"),
        State::Outdated => l("يحتاج تحديثًا", "Needs updating"),
        State::Unreadable(_) => l("تعذّرت قراءة إعداداته", "Its settings can't be read"),
    }
}

pub(crate) fn show(app: &mut WordApp, ctx: &egui::Context) {
    if !app.connect_ai.open {
        return;
    }
    let mut open = true;
    let mut close = false;
    let shown = egui::Window::new(title())
        .id(egui::Id::unique("connect-ai-window"))
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .default_width(440.0)
        .show(ctx, |ui| {
            ui.set_max_width(440.0);
            body(app, ui);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(l("إغلاق", "Close")).clicked() {
                    close = true;
                }
            });
        });
    // Mirrored as a whole in the Arabic interface, like every dialog.
    if let Some(shown) = shown {
        crate::mirror::overlay(ctx, shown.response.layer_id);
    }
    if !open || close {
        app.connect_ai.open = false;
    }
}

#[cfg(target_arch = "wasm32")]
fn body(_app: &mut WordApp, ui: &mut egui::Ui) {
    ui.label(l(
        "الربط مع تطبيقات الذكاء الاصطناعي متاح في تطبيق كلمة على الجهاز، لا في المتصفح.",
        "Connecting AI apps is available in the desktop app, not in the web browser.",
    ));
}

#[cfg(not(target_arch = "wasm32"))]
fn body(app: &mut WordApp, ui: &mut egui::Ui) {
    use ai_connect::State;
    ui.label(l(
        "اربط كلمة بتطبيق ذكاء اصطناعي، فيكتب وينسّق ويحفظ مستنداتك حين تطلب منه ذلك بكلامك. اضغط «ربط» بجانب التطبيق ثم أعد تشغيله.",
        "Connect Kalima to an AI app so it can write, format and save your documents when you ask it to. Press Connect next to the app, then restart it.",
    ));
    ui.add_space(6.0);
    let Some(server) = server() else {
        ui.colored_label(
            ui.visuals().error_fg_color,
            l(
                "لم يُعثر على خادم MCP ‏(kalima-cli) بجانب كلمة. نزّل كلمة من جديد.",
                "The MCP server (kalima-cli) isn't next to Kalima. Download Kalima again.",
            ),
        );
        return;
    };
    if ai_connect::temporary_location(&server.command) {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            l(
                "انقل كلمة إلى مجلد التطبيقات (Applications) وافتحه من هناك أولًا، وإلا توقّف الربط بعد إغلاقه.",
                "Move Kalima to the Applications folder and open it from there first, or the connection stops working once it closes.",
            ),
        );
        ui.add_space(4.0);
    }
    let Some(dirs) = ai_connect::Dirs::system() else { return };
    let mut action: Option<(ai_connect::Client, bool)> = None;
    let installed: Vec<_> = app.connect_ai.rows.iter().filter(|(_, s)| *s != State::NotInstalled).cloned().collect();
    if installed.is_empty() {
        ui.label(l(
            "لم نجد على جهازك تطبيق ذكاء اصطناعي نعرفه. ثبّت أحدها، مثل Claude، ثم افتح هذه النافذة من جديد.",
            "No AI app we know of is installed here. Install one, such as Claude, then open this window again.",
        ));
    } else {
        egui::Grid::new("connect_ai_grid").num_columns(3).spacing([16.0, 8.0]).show(ui, |ui| {
            for (client, state) in &installed {
                ui.label(egui::RichText::new(client.name()).strong());
                let label = ui.label(state_label(state));
                if let State::Unreadable(why) = state {
                    label.on_hover_text(why);
                }
                ui.horizontal(|ui| match state {
                    State::NotConnected => {
                        if ui.button(l("ربط", "Connect")).clicked() {
                            action = Some((*client, false));
                        }
                    }
                    State::Outdated => {
                        if ui.button(l("تحديث", "Update")).clicked() {
                            action = Some((*client, false));
                        }
                    }
                    State::Connected => {
                        if ui.button(l("إلغاء الربط", "Disconnect")).clicked() {
                            action = Some((*client, true));
                        }
                    }
                    State::NotInstalled | State::Unreadable(_) => {}
                });
                ui.end_row();
            }
        });
    }
    let missing: Vec<&str> = app.connect_ai.rows.iter().filter(|(_, s)| *s == State::NotInstalled).map(|(c, _)| c.name()).collect();
    if !missing.is_empty() {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(format!("{} {}", l("غير مثبتة على جهازك:", "Not installed here:"), missing.join(l("، ", ", ")))).small().weak());
    }
    if let Some((client, disconnect)) = action {
        let result = if disconnect {
            ai_connect::disconnect(client, &dirs, &server).map(|()| l("أُلغي الربط.", "Disconnected.").to_string())
        } else {
            ai_connect::connect(client, &dirs, &server).map(|done| match done {
                ai_connect::Done::Saved if client.needs_restart() => {
                    format!("{} {}", l("تم الربط. أعد تشغيل", "Connected. Restart"), client.name())
                }
                ai_connect::Done::Saved => l("تم الربط.", "Connected.").to_string(),
                ai_connect::Done::OpenLink(link) => {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(link));
                    format!("{} {}", l("وافق على الإضافة في", "Confirm the install in"), client.name())
                }
            })
        };
        app.connect_ai.message = Some(match result {
            Ok(text) => (text, false),
            Err(e) => {
                log::warn!("connecting {}: {e}", client.name());
                (format!("{} {e}", l("تعذّر الربط:", "Couldn't connect:")), true)
            }
        });
        refresh(&mut app.connect_ai);
    }
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(l(
            "نصيحة: اذكر kalima في طلبك ليستخدم التطبيقُ كلمة لا Microsoft Word، مثل: «استخدم أدوات kalima لإنشاء مستند جديد…».",
            "Tip: say kalima in your request so the AI app uses Kalima, not Microsoft Word, e.g. \"Use the kalima tools to create a new document…\".",
        ))
        .small(),
    );
    if let Some((text, bad)) = &app.connect_ai.message {
        ui.add_space(6.0);
        if *bad {
            ui.colored_label(ui.visuals().error_fg_color, text);
        } else {
            ui.label(egui::RichText::new(text).strong());
        }
    }
    ui.add_space(8.0);
    ui.separator();
    ui.label(l(
        "تطبيق آخر يدعم MCP؟ انسخ الإعداد والصقه في إعدادات خوادم MCP فيه.",
        "Another MCP app? Copy the settings and paste them into its MCP servers settings.",
    ));
    if ui.button(l("نسخ الإعداد", "Copy Settings")).clicked() {
        ui.ctx().copy_text(server.snippet());
        app.connect_ai.message = Some((l("نُسخ الإعداد.", "Settings copied.").to_string(), false));
    }
}
