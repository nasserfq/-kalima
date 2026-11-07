//! macOS: the app menu (About, Services, Hide, Hide Others, Show All, Quit). winit puts it up in
//! English and names the app after its executable (the upstream name), so this retitles each item
//! by its action: in Arabic while the interface is Arabic, and with the app's own Arabic name.

use objc2::MainThreadMarker;
use objc2::runtime::Sel;
use objc2::sel;
use objc2_app_kit::NSApplication;
use objc2_foundation::NSString;

/// The app menu's items, in `arabic` or English, naming the app `app`. The Services submenu is
/// the item without an action.
pub fn titles(arabic: bool, app: &str) -> [(Option<Sel>, String); 6] {
    let [about, services, hide, others, all, quit] = if arabic {
        ["حول {app}", "الخدمات", "إخفاء {app}", "إخفاء الآخرين", "إظهار الكل", "إنهاء {app}"]
    } else {
        ["About {app}", "Services", "Hide {app}", "Hide Others", "Show All", "Quit {app}"]
    };
    let name = |s: &str| s.replace("{app}", app);
    [
        (Some(sel!(orderFrontStandardAboutPanel:)), name(about)),
        (None, name(services)),
        (Some(sel!(hide:)), name(hide)),
        (Some(sel!(hideOtherApplications:)), name(others)),
        (Some(sel!(unhideAllApplications:)), name(all)),
        (Some(sel!(terminate:)), name(quit)),
    ]
}

thread_local! {
    /// The language the menu was last titled in (true: Arabic); `None` until the menu exists.
    static APPLIED: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Keeps the app menu in the interface language: retitles it when the language changed (or the
/// menu was not up yet). Cheap to call every frame.
pub fn sync(arabic: bool, app: &str) {
    if APPLIED.get() == Some(arabic) {
        return;
    }
    if retitle(arabic, app) {
        APPLIED.set(Some(arabic));
    }
}

/// Retitles the first menu of the menu bar. False when there is no menu bar yet.
fn retitle(arabic: bool, app: &str) -> bool {
    let Some(mtm) = MainThreadMarker::new() else { return false };
    let Some(bar) = NSApplication::sharedApplication(mtm).mainMenu() else { return false };
    let Some(menu) = bar.itemAtIndex(0).and_then(|item| item.submenu()) else { return false };
    menu.setTitle(&NSString::from_str(app));
    let titles = titles(arabic, app);
    for i in 0..menu.numberOfItems() {
        let Some(item) = menu.itemAtIndex(i) else { continue };
        if item.isSeparatorItem() {
            continue;
        }
        let action = if item.submenu().is_some() { None } else { item.action() };
        if let Some((_, title)) = titles.iter().find(|(sel, _)| *sel == action) {
            let title = NSString::from_str(title);
            item.setTitle(&title);
            if let Some(sub) = item.submenu() {
                sub.setTitle(&title);
            }
        }
    }
    true
}
