// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use vauchi_app::i18n::Locale;

/// Detect locale from environment variables (LC_ALL, LC_MESSAGES, LANG).
///
/// Mirrors `tui/src/app/mod.rs::detect_locale` so all native Linux/Unix
/// frontends agree on the precedence rule. Extracts the language-code
/// portion (e.g. `de_CH.UTF-8` → `de`) and falls back to
/// `Locale::default()` (English) when no variable is set, the value is
/// `C` / `POSIX`, or the code does not map to a supported locale.
pub fn detect_locale() -> Locale {
    detect_locale_from(|name| std::env::var(name).ok())
}

fn detect_locale_from(mut value_for: impl FnMut(&str) -> Option<String>) -> Locale {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(val) = value_for(var) {
            let code = val.split('_').next().unwrap_or(&val);
            let code = code.split('.').next().unwrap_or(code);
            if !code.is_empty()
                && code != "C"
                && code != "POSIX"
                && let Some(locale) = Locale::from_code(code)
            {
                return locale;
            }
        }
    }
    Locale::default()
}

// INLINE_TEST_REQUIRED: linux-gtk is a binary crate (no lib.rs);
// integration tests cannot reach this module from `tests/`.
#[cfg(test)]
mod tests {
    use super::*;

    fn detect_test_locale(values: &[(&str, &str)]) -> Locale {
        detect_locale_from(|name| {
            values
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_string())
        })
    }

    // @internal
    #[test]
    fn lc_all_takes_precedence_over_lang() {
        assert_eq!(
            detect_test_locale(&[("LC_ALL", "de_DE.UTF-8"), ("LANG", "fr_FR.UTF-8")]),
            Locale::German
        );
    }

    // @internal
    #[test]
    fn lc_messages_used_when_lc_all_unset() {
        assert_eq!(
            detect_test_locale(&[("LC_MESSAGES", "es_ES.UTF-8"), ("LANG", "it_IT.UTF-8")]),
            Locale::Spanish
        );
    }

    // @internal
    #[test]
    fn lang_used_when_no_lc_var_set() {
        assert_eq!(
            detect_test_locale(&[("LANG", "it_CH.UTF-8")]),
            Locale::Italian
        );
    }

    // @internal
    #[test]
    fn c_locale_falls_back_to_default() {
        assert_eq!(detect_test_locale(&[("LC_ALL", "C")]), Locale::default());
    }

    // @internal
    #[test]
    fn posix_locale_falls_back_to_default() {
        assert_eq!(
            detect_test_locale(&[("LC_MESSAGES", "POSIX")]),
            Locale::default()
        );
    }

    // @internal
    #[test]
    fn no_env_vars_returns_default() {
        assert_eq!(detect_test_locale(&[]), Locale::default());
    }

    // @internal
    #[test]
    fn unknown_language_falls_back_to_default() {
        assert_eq!(
            detect_test_locale(&[("LANG", "zz_ZZ.UTF-8")]),
            Locale::default()
        );
    }
}
