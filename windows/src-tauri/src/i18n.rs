// Interface language for the few strings Rust shows itself: the tray menu, the
// Settings window title and error messages that reach the UI. Same rule as the
// webview (src/i18n): any Portuguese Windows UI language → pt-BR, else English.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    PtBr,
}

#[cfg(not(test))]
pub fn lang() -> Lang {
    use std::sync::OnceLock;
    /// Primary language id of Portuguese (LANG_PORTUGUESE).
    const LANG_PORTUGUESE: u16 = 0x16;
    static LANG: OnceLock<Lang> = OnceLock::new();
    *LANG.get_or_init(|| {
        let id = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
        if id & 0x3ff == LANG_PORTUGUESE { Lang::PtBr } else { Lang::En }
    })
}

/// Tests assert on English messages whatever the machine's language.
#[cfg(test)]
pub fn lang() -> Lang {
    Lang::En
}

pub fn pt() -> bool {
    lang() == Lang::PtBr
}

/// Picks the English or the Portuguese string.
pub fn t(en: &'static str, pt_br: &'static str) -> &'static str {
    if pt() { pt_br } else { en }
}
