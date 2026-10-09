//! The languages a post can be tagged with, and looking one up by code.

/// A language a post can be tagged with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    pub code: &'static str,
    pub name: &'static str,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

const fn lang(code: &'static str, name: &'static str) -> Language {
    Language { code, name }
}

/// The languages offered when posting, by English name.
pub const LANGUAGES: &[Language] = &[
    lang("ar", "Arabic"),
    lang("ca", "Catalan"),
    lang("zh", "Chinese"),
    lang("cs", "Czech"),
    lang("da", "Danish"),
    lang("nl", "Dutch"),
    lang("en", "English"),
    lang("fi", "Finnish"),
    lang("fr", "French"),
    lang("de", "German"),
    lang("el", "Greek"),
    lang("he", "Hebrew"),
    lang("hi", "Hindi"),
    lang("hu", "Hungarian"),
    lang("id", "Indonesian"),
    lang("it", "Italian"),
    lang("ja", "Japanese"),
    lang("ko", "Korean"),
    lang("no", "Norwegian"),
    lang("fa", "Persian"),
    lang("pl", "Polish"),
    lang("pt", "Portuguese"),
    lang("ro", "Romanian"),
    lang("ru", "Russian"),
    lang("es", "Spanish"),
    lang("sv", "Swedish"),
    lang("th", "Thai"),
    lang("tr", "Turkish"),
    lang("uk", "Ukrainian"),
    lang("vi", "Vietnamese"),
];

pub fn language(code: &str) -> Option<Language> {
    LANGUAGES.iter().copied().find(|l| l.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_are_looked_up_by_code() {
        assert_eq!(language("fr").map(|l| l.name), Some("French"));
        assert!(language("xx").is_none());
    }
}
