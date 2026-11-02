//! Software updates, background check at startup, and startup prompt modal.

use crate::WordApp;

#[cfg(not(target_arch = "wasm32"))]
use craft_update::dialog::{CheckResultDialog, CheckResultResponse};
#[cfg(not(target_arch = "wasm32"))]
use craft_update::updater::{Action, Status, Updater};

pub(crate) struct Runtime {
    #[cfg(not(target_arch = "wasm32"))]
    updater: Updater,
    #[cfg(not(target_arch = "wasm32"))]
    result_dialog: CheckResultDialog,
    /// The desktop app turns the check at start on; tests and the web build leave it off.
    #[cfg(not(target_arch = "wasm32"))]
    startup_check: bool,
    /// The context of the last frame, to wake the UI when a check is queued from a menu callback.
    #[cfg(not(target_arch = "wasm32"))]
    repaint: Option<egui::Context>,
    pub(crate) manual_window_open: bool,
    #[cfg(target_arch = "wasm32")]
    pub(crate) status_message: Option<String>,
    /// كلمة's own check: a newer release found at start (its number), until dismissed.
    #[cfg(not(target_arch = "wasm32"))]
    kalima: std::sync::Arc<std::sync::Mutex<Option<u32>>>,
    #[cfg(not(target_arch = "wasm32"))]
    kalima_checked: bool,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            updater: Updater::new(&crate::UPDATE_APP, env!("CARGO_PKG_VERSION")),
            #[cfg(not(target_arch = "wasm32"))]
            result_dialog: CheckResultDialog::new(&crate::UPDATE_APP, env!("CARGO_PKG_VERSION")),
            #[cfg(not(target_arch = "wasm32"))]
            startup_check: false,
            #[cfg(not(target_arch = "wasm32"))]
            repaint: None,
            manual_window_open: false,
            #[cfg(target_arch = "wasm32")]
            status_message: None,
            #[cfg(not(target_arch = "wasm32"))]
            kalima: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            kalima_checked: false,
        }
    }
}

impl WordApp {
    /// Whether the app asks for a newer release when it starts (off until the desktop app turns it on).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_startup_update_check(&mut self, on: bool) {
        self.updates_rt.startup_check = on;
    }
}

/// كلمة's releases, where Help › Check for Updates goes (WordCraft's updater would install WordCraft).
pub const RELEASES_URL: &str = "https://github.com/nasserfq/kalima/releases/latest";

pub(crate) fn check(app: &mut WordApp) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ctx) = &app.updates_rt.repaint {
        ctx.open_url(egui::OpenUrl::new_tab(RELEASES_URL));
    }
    #[cfg(target_arch = "wasm32")]
    {
        app.updates_rt.manual_window_open = true;
        app.updates_rt.status_message = Some(tl!("Update check is not supported in the web browser.").to_string());
    }
}

pub(crate) fn tick(app: &mut WordApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let rt = &mut app.updates_rt;
        rt.repaint = Some(ctx.clone());
        // WordCraft's updater stays idle: it checks WordCraft's releases and would install
        // WordCraft over كلمة. كلمة checks its own releases instead.
        if rt.startup_check && !std::mem::replace(&mut rt.kalima_checked, true) {
            check_kalima(rt.kalima.clone(), ctx.clone());
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (app, ctx);
}

pub(crate) fn show(app: &mut WordApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // The restart helper is scheduled before the app closes, so no update starts over unsaved work.
        let clean = !(app.session.dirty || app.save_unconfirmed() || app.printing.busy() || crate::panes::has_comment_drafts(app));
        match app.updates_rt.updater.show_if(ctx, clean) {
            Action::OpenUrl(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
            Action::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Action::None => {}
        }
        show_kalima(&app.updates_rt.kalima, ctx);
        // The dialog answers Help ▸ Check for Updates; a found update leaves the status idle and the prompt above takes over.
        let rt = &mut app.updates_rt;
        if rt.manual_window_open {
            match rt.result_dialog.show(ctx, rt.updater.status()) {
                CheckResultResponse::None => {}
                CheckResultResponse::Close => {
                    rt.manual_window_open = false;
                    rt.updater.dismiss_status();
                }
                CheckResultResponse::CheckAgain => rt.updater.check_now(ctx),
            }
            // An update was found: the prompt replaces the dialog.
            if matches!(rt.updater.status(), Status::Idle) {
                rt.manual_window_open = false;
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    if app.updates_rt.manual_window_open {
        let mut open = true;
        egui::Window::new(tl!("Check for Updates"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                if let Some(msg) = &app.updates_rt.status_message {
                    ui.label(msg);
                }
            });
        if !open {
            app.updates_rt.manual_window_open = false;
            app.updates_rt.status_message = None;
        }
    }
}

/// This build's release number (`kalima-<n>`), set by the release workflow; none in local builds,
/// which don't check.
#[cfg(not(target_arch = "wasm32"))]
const KALIMA_BUILD: Option<&str> = option_env!("KALIMA_BUILD");

/// The release number in a GitHub "latest release" answer (`"tag_name": "kalima-16"`).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn kalima_release(json: &str) -> Option<u32> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    v.get("tag_name")?.as_str()?.strip_prefix("kalima-")?.parse().ok()
}

/// Ask GitHub, in the background, for كلمة's latest release; keep its number when it's newer.
#[cfg(not(target_arch = "wasm32"))]
fn check_kalima(found: std::sync::Arc<std::sync::Mutex<Option<u32>>>, ctx: egui::Context) {
    let Some(this) = KALIMA_BUILD.and_then(|b| b.parse::<u32>().ok()) else { return };
    std::thread::spawn(move || {
        let latest = craft_update::Http::new(&format!("Kalima/{this}"))
            .and_then(|h| h.get_text("https://api.github.com/repos/nasserfq/kalima/releases/latest"))
            .map(|t| t.as_deref().and_then(kalima_release));
        match latest {
            Ok(Some(n)) if n > this => {
                if let Ok(mut f) = found.lock() {
                    *f = Some(n);
                }
                ctx.request_repaint();
            }
            Ok(_) => {}
            Err(e) => log::info!("كلمة update check: {e}"),
        }
    });
}

/// The prompt for a newer كلمة: download it from the releases page, or later.
#[cfg(not(target_arch = "wasm32"))]
fn show_kalima(found: &std::sync::Mutex<Option<u32>>, ctx: &egui::Context) {
    let Some(n) = found.lock().ok().and_then(|f| *f) else { return };
    let mut close = false;
    egui::Modal::new(egui::Id::unique("kalima.update")).show(ctx, |ui| {
        ui.set_max_width(360.0);
        ui.heading("يتوفر إصدار جديد من كلمة");
        ui.label(format!("الإصدار {n} متاح للتنزيل. نزّله واستبدل به النسخة الحالية، وتبقى إعداداتك ومستنداتك كما هي."));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("تنزيل").clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(RELEASES_URL));
                close = true;
            }
            if ui.button("لاحقًا").clicked() {
                close = true;
            }
        });
    });
    if close && let Ok(mut f) = found.lock() {
        *f = None;
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    #[test]
    fn reads_the_kalima_release_number() {
        assert_eq!(super::kalima_release(r#"{"tag_name":"kalima-16","name":"x"}"#), Some(16));
        assert_eq!(super::kalima_release(r#"{"tag_name":"v0.14.0"}"#), None);
        assert_eq!(super::kalima_release("not json"), None);
    }
}
