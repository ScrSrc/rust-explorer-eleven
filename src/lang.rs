//! Interface language.
//!
//! Strings in the source code stay in Russian and act as translation keys;
//! [`t`] looks them up for the active language and falls back to the key
//! itself when a translation is missing. The active language is a
//! process-wide atomic, so every helper (including ones without app access)
//! can localise its texts.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    /// Русский
    Ru,
    /// English
    En,
    /// Українська
    Uk,
    /// Қазақша
    Kk,
    /// 中文
    Zh,
    /// Español
    Es,
    /// Português
    Pt,
    /// Türkçe
    Tr,
}

pub const ALL_LANGS: [Lang; 8] = [
    Lang::Ru,
    Lang::En,
    Lang::Uk,
    Lang::Kk,
    Lang::Zh,
    Lang::Es,
    Lang::Pt,
    Lang::Tr,
];

impl Lang {
    /// Stable INI identifier.
    pub fn code(self) -> &'static str {
        match self {
            Lang::Ru => "ru",
            Lang::En => "en",
            Lang::Uk => "uk",
            Lang::Kk => "kk",
            Lang::Zh => "zh",
            Lang::Es => "es",
            Lang::Pt => "pt",
            Lang::Tr => "tr",
        }
    }

    /// Self-named label for the language menu (always in its own language).
    pub fn native(self) -> &'static str {
        match self {
            Lang::Ru => "Русский",
            Lang::En => "English",
            Lang::Uk => "Українська",
            Lang::Kk => "Қазақша",
            Lang::Zh => "中文",
            Lang::Es => "Español",
            Lang::Pt => "Português",
            Lang::Tr => "Türkçe",
        }
    }

    pub fn parse(value: &str) -> Option<Lang> {
        ALL_LANGS.iter().copied().find(|l| l.code() == value)
    }

    fn index(self) -> usize {
        ALL_LANGS.iter().position(|l| *l == self).unwrap_or(0)
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

/// The language used for every [`t`] lookup.
pub fn current() -> Lang {
    let index = (CURRENT.load(Ordering::Relaxed) as usize).min(ALL_LANGS.len() - 1);
    ALL_LANGS[index]
}

/// Switches the active language (also persisted through `Preferences`).
pub fn set_current(lang: Lang) {
    CURRENT.store(lang.index() as u8, Ordering::Relaxed);
}

/// Translates `key` (a Russian source string). Russian falls straight
/// through; other languages fall back to the Russian key when their table
/// has no entry.
pub fn t(key: &'static str) -> &'static str {
    match current() {
        Lang::Ru => key,
        Lang::En => dict::en::en(key),
        Lang::Uk => dict::uk::uk(key),
        Lang::Kk => dict::kk::kk(key),
        Lang::Zh => dict::zh::zh(key),
        Lang::Es => dict::es::es(key),
        Lang::Pt => dict::pt::pt(key),
        Lang::Tr => dict::tr::tr(key),
    }
}

/// Substitutes placeholders of a translated template: every `{name}` or `{}`
/// consumes the next entry of `args`, in order. Replaces using `format!`
/// with a runtime string, which Rust does not allow.
pub fn f(template: &'static str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len() + 32);
    let mut tail = template;
    let mut index = 0usize;
    while let Some(start) = tail.find('{') {
        let Some(end_rel) = tail[start..].find('}') else {
            break;
        };
        let end = start + end_rel;
        out.push_str(&tail[..start]);
        if let Some(arg) = args.get(index) {
            out.push_str(arg);
            index += 1;
        }
        tail = &tail[end + 1..];
    }
    out.push_str(tail);
    out
}

mod dict {
    //! Translation tables. Each function is a `match key { ... }` dictionary;
    //! any key missing from the table stays Russian (the source strings).
    pub mod en;
    pub mod uk;
    pub mod kk;
    pub mod zh;
    pub mod es;
    pub mod pt;
    pub mod tr;
}
