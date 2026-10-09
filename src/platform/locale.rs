//! The language the system is set to: the user's locale on Windows and macOS, and
//! `LC_ALL`, `LC_MESSAGES` or `LANG` on Linux (through `sys-locale`).

use std::sync::OnceLock;

/// The system's language as a lowercase code such as "en" or "pt", without its
/// region. `None` when the system doesn't say. Read once; views ask for it often.
pub fn language() -> Option<String> {
    static LANGUAGE: OnceLock<Option<String>> = OnceLock::new();
    LANGUAGE
        .get_or_init(|| sys_locale::get_locale().and_then(|tag| base(&tag)))
        .clone()
}

/// "pt-BR", "pt_BR.UTF-8" and "pt@euro" are all "pt". The "C" and "POSIX" locales
/// name no language.
fn base(tag: &str) -> Option<String> {
    let code = tag.split(['-', '_', '.', '@']).next()?.to_lowercase();
    (!code.is_empty() && code != "c" && code != "posix").then_some(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_become_language_codes() {
        assert_eq!(base("pt-BR").as_deref(), Some("pt"));
        assert_eq!(base("de_DE.UTF-8").as_deref(), Some("de"));
        assert_eq!(base("fr@euro").as_deref(), Some("fr"));
        assert_eq!(base("EN").as_deref(), Some("en"));
        assert_eq!(base("C.UTF-8"), None);
        assert_eq!(base("POSIX"), None);
        assert_eq!(base(""), None);
    }
}
