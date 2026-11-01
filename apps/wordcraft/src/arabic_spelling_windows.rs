//! Arabic spelling for كلمة through Windows' own spell checker (the Spell Checking API), registered
//! with `wordcraft_proof::ar` at startup. Windows checks Arabic once Arabic is added in Settings ›
//! Time & language › Language & region.

use std::sync::OnceLock;

use wordcraft_engine::proof::ar;

/// The language tag Windows checks Arabic under.
static LANG: OnceLock<&'static str> = OnceLock::new();

/// Register the system's Arabic spell checker, when Windows has one.
pub fn register() {
    let Some(lang) = ["ar-SA", "ar"].into_iter().find(|l| kalima_win_spell::is_supported(l)) else {
        log::info!("Arabic spelling: Windows has no Arabic spell checker (add Arabic in Language settings)");
        return;
    };
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
