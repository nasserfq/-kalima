//! Windows' own spell checker (the Spell Checking API, Windows 8 and later) behind a safe API.
//! كلمة registers it for Arabic. Windows checks a language once that language is added in
//! Settings › Time & language › Language & region.
//!
//! Each thread gets its own COM initialization and its own checker, so the background proofing
//! passes can call this from any thread. On other systems every call answers "no checker".

/// Install Windows' spell checking for `lang` (its "Basic typing" language feature, from Windows
/// Update), without adding it as a display or keyboard language. Windows asks the user to allow
/// it (administrator prompt); this waits until the install ends. True when `lang` can then be
/// checked.
pub fn install(lang: &str) -> bool {
    imp::install(lang);
    is_supported(lang)
}

/// Can Windows check spelling in `lang` (a BCP 47 tag such as `ar-SA`)?
pub fn is_supported(lang: &str) -> bool {
    imp::with_checker(lang, |_| ()).is_some()
}

/// Is `word` spelled correctly in `lang`? `None` when Windows can't check `lang`.
pub fn is_correct(lang: &str, word: &str) -> Option<bool> {
    imp::with_checker(lang, |c| imp::check(c, word)).flatten()
}

/// Windows' suggestions for `word` in `lang`, best first, at most `max`.
pub fn suggest(lang: &str, word: &str, max: usize) -> Vec<String> {
    imp::with_checker(lang, |c| imp::suggest(c, word, max)).unwrap_or_default()
}

#[cfg(windows)]
mod imp {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use windows::Win32::Globalization::{ISpellChecker, ISpellCheckerFactory, ISpellingError, SpellCheckerFactory};
    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree};
    use windows::core::{HRESULT, HSTRING, PWSTR};

    thread_local! {
        /// This thread's checkers by language. Languages Windows can't check aren't kept, so one
        /// installed while the app runs is picked up.
        static CHECKERS: RefCell<HashMap<String, ISpellChecker>> = RefCell::new(HashMap::new());
    }

    fn create(lang: &str) -> Option<ISpellChecker> {
        let tag = HSTRING::from(lang);
        // SAFETY: plain COM calls with valid arguments. COM stays initialized on this thread for
        // as long as it runs (an extra or mismatched init only returns an error, ignored here).
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let factory: ISpellCheckerFactory = CoCreateInstance(&SpellCheckerFactory, None, CLSCTX_INPROC_SERVER).ok()?;
            if !factory.IsSupported(&tag).ok()?.as_bool() {
                return None;
            }
            factory.CreateSpellChecker(&tag).ok()
        }
    }

    pub fn with_checker<R>(lang: &str, f: impl FnOnce(&ISpellChecker) -> R) -> Option<R> {
        CHECKERS.with(|map| {
            let mut map = map.borrow_mut();
            if !map.contains_key(lang) {
                map.insert(lang.to_string(), create(lang)?);
            }
            map.get(lang).map(f)
        })
    }

    pub fn install(lang: &str) {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        if !lang.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return;
        }
        let inner = format!("Add-WindowsCapability -Online -Name Language.Basic~~~{lang}~0.0.1.0");
        let outer = format!("Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '-NoProfile','-Command','{inner}'");
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &outer])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
        if let Err(e) = status {
            log::warn!("spell checking install for {lang}: {e}");
        }
    }

    pub fn check(checker: &ISpellChecker, word: &str) -> Option<bool> {
        // SAFETY: COM calls on a live checker; `Next` writes one interface pointer into `error`.
        unsafe {
            let errors = checker.Check(&HSTRING::from(word)).ok()?;
            let mut error: Option<ISpellingError> = None;
            // S_OK: an error was found. S_FALSE: none.
            let hr = errors.Next(&mut error);
            if hr.is_err() {
                return None;
            }
            Some(!(hr == HRESULT(0) && error.is_some()))
        }
    }

    pub fn suggest(checker: &ISpellChecker, word: &str, max: usize) -> Vec<String> {
        let mut out = Vec::new();
        // SAFETY: COM calls on a live checker. Each string `Next` returns is ours to read once and
        // free with CoTaskMemFree.
        unsafe {
            let Ok(list) = checker.Suggest(&HSTRING::from(word)) else { return out };
            while out.len() < max {
                let mut item = [PWSTR::null()];
                let mut fetched = 0u32;
                let hr = list.Next(&mut item, Some(&mut fetched));
                if hr != HRESULT(0) || fetched == 0 || item[0].is_null() {
                    break;
                }
                if let Ok(s) = item[0].to_string() {
                    out.push(s);
                }
                CoTaskMemFree(Some(item[0].0 as *const _));
            }
        }
        out
    }
}

#[cfg(not(windows))]
mod imp {
    pub enum Never {}
    pub fn with_checker<R>(_lang: &str, _f: impl FnOnce(&Never) -> R) -> Option<R> {
        None
    }
    pub fn install(_lang: &str) {}
    pub fn check(c: &Never, _word: &str) -> Option<bool> {
        match *c {}
    }
    pub fn suggest(c: &Never, _word: &str, _max: usize) -> Vec<String> {
        match *c {}
    }
}
