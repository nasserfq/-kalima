//! Arabic spelling for كلمة through Windows' own spell checker (the Spell Checking API), registered
//! with `wordcraft_proof::ar` at startup. Without one, كلمة installs Windows' Arabic spell checking
//! on first start.

use std::sync::OnceLock;

use wordcraft_engine::proof::ar;

/// The language tag Windows checks Arabic under.
static LANG: OnceLock<&'static str> = OnceLock::new();

/// Register the system's Arabic spell checker. When Windows has none, install it once (in the
/// background; Windows asks to allow it) and register it when that's done. Turned off with
/// Options › Proofing › Check spelling as you type, like every language.
pub fn register() {
    if let Some(lang) = ["ar-SA", "ar"].into_iter().find(|l| kalima_win_spell::is_supported(l)) {
        set(lang);
        return;
    }
    // Only one try: a declined prompt or a blocked install isn't asked again at every start.
    let Some(marker) = wordcraft_control_key::settings_dir().map(|d| d.join("arabic-spelling-install-tried")) else { return };
    if marker.exists() {
        log::info!("Arabic spelling: Windows has no Arabic spell checker (add Arabic in Language settings)");
        return;
    }
    let _ = std::fs::create_dir_all(marker.parent().unwrap_or(&marker));
    let _ = std::fs::write(&marker, b"");
    std::thread::spawn(|| {
        log::info!("Arabic spelling: installing Windows' Arabic spell checker");
        if kalima_win_spell::install("ar-SA") {
            set("ar-SA");
        } else {
            log::info!("Arabic spelling: install declined or failed");
        }
    });
}

fn set(lang: &'static str) {
    let _ = LANG.set(lang);
    ar::set_speller(ar::Speller { is_correct, suggest });
}

fn lang() -> &'static str {
    LANG.get().copied().unwrap_or("ar-SA")
}

fn is_correct(word: &str) -> bool {
    kalima_win_spell::is_correct(lang(), word).unwrap_or(true)
}

fn suggest(word: &str, max: usize) -> Vec<String> {
    kalima_win_spell::suggest(lang(), word, max)
}
