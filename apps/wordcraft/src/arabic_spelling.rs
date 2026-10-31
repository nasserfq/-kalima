//! Arabic spelling for كلمة through macOS's own spell checker (NSSpellChecker), registered with
//! `wordcraft_proof::ar` at startup. NSSpellChecker isn't tied to the main thread, so the
//! background proofing passes may call it.

use objc2_app_kit::NSSpellChecker;
use objc2_foundation::{NSRange, NSString};
use wordcraft_engine::proof::ar;

/// Register the system's Arabic spell checker, when macOS has one.
pub fn register() {
    let checker = NSSpellChecker::sharedSpellChecker();
    if !checker.setLanguage(&NSString::from_str("ar")) {
        log::info!("Arabic spelling: macOS has no Arabic spell checker");
        return;
    }
    ar::set_speller(ar::Speller { is_correct, suggest });
}

fn is_correct(word: &str) -> bool {
    objc2::rc::autoreleasepool(|_| {
        let checker = NSSpellChecker::sharedSpellChecker();
        checker.checkSpellingOfString_startingAt(&NSString::from_str(word), 0).length == 0
    })
}

fn suggest(word: &str, max: usize) -> Vec<String> {
    objc2::rc::autoreleasepool(|_| {
        let checker = NSSpellChecker::sharedSpellChecker();
        let s = NSString::from_str(word);
        let lang = NSString::from_str("ar");
        let Some(guesses) = checker.guessesForWordRange_inString_language_inSpellDocumentWithTag(NSRange::new(0, s.length()), &s, Some(&lang), 0) else {
            return Vec::new();
        };
        (0..guesses.count().min(max)).map(|i| guesses.objectAtIndex(i).to_string()).collect()
    })
}
