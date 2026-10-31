//! Arabic spelling through the platform's own spell checker, which the app registers with
//! [`set_speller`] (macOS's NSSpellChecker in كلمة). With none registered (other systems, tests)
//! every Arabic word is accepted, as for a language without a dictionary. No Arabic grammar
//! rules or hyphenation.

use std::sync::OnceLock;

/// A spell checker for Arabic words.
#[derive(Clone, Copy)]
pub struct Speller {
    /// Is the word spelled correctly?
    pub is_correct: fn(&str) -> bool,
    /// Suggestions for a misspelled word, best first, at most the given number.
    pub suggest: fn(&str, usize) -> Vec<String>,
}

static SPELLER: OnceLock<Speller> = OnceLock::new();

/// Use this spell checker for Arabic from now on (the first one registered stays).
pub fn set_speller(s: Speller) {
    let _ = SPELLER.set(s);
}

/// Is Arabic spelling checked (a spell checker is registered)?
pub fn has_speller() -> bool {
    SPELLER.get().is_some()
}

/// Is `c` an Arabic-script letter (Arabic, its supplements and presentation forms)?
pub fn is_arabic_letter(c: char) -> bool {
    c.is_alphabetic() && matches!(c, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}' | '\u{FB50}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}')
}

/// Is `word` written in Arabic script? It is then spelled in Arabic, whatever the run's language.
pub fn is_arabic_word(word: &str) -> bool {
    word.chars().any(is_arabic_letter)
}

/// Is `word` spelled correctly in Arabic? True when no spell checker is registered.
pub fn is_correct(word: &str) -> bool {
    SPELLER.get().is_none_or(|s| (s.is_correct)(word))
}

/// Spelling suggestions for an Arabic word (none without a spell checker).
pub fn suggest(word: &str, max: usize) -> Vec<String> {
    SPELLER.get().map(|s| (s.suggest)(word, max)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn arabic_words_are_told_apart() {
        assert!(super::is_arabic_word("مرحبا"));
        assert!(!super::is_arabic_word("hello"));
        assert!(!super::is_arabic_word("123"));
    }
}
