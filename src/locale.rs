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
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
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
    use std::sync::Mutex;

    // `std::env::set_var` / `remove_var` mutate process-wide state — guard
    // these tests with a mutex so parallel runs do not stomp each other.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<R>(vars: &[(&str, Option<&str>)], f: impl FnOnce() -> R) -> R {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let saved: Vec<(String, Option<String>)> = vars
            .iter()
            .map(|(k, _)| ((*k).to_string(), std::env::var(*k).ok()))
            .collect();
        for (k, _) in vars {
            unsafe { std::env::remove_var(k) };
        }
        for (k, v) in vars {
            if let Some(val) = v {
                unsafe { std::env::set_var(k, val) };
            }
        }
        let r = f();
        for (k, prior) in &saved {
            match prior {
                Some(v) => unsafe { std::env::set_var(k, v) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
        r
    }

    // @internal
    #[test]
    fn lc_all_takes_precedence_over_lang() {
        with_env(
            &[
                ("LC_ALL", Some("de_DE.UTF-8")),
                ("LC_MESSAGES", None),
                ("LANG", Some("fr_FR.UTF-8")),
            ],
            || {
                assert_eq!(detect_locale(), Locale::German);
            },
        );
    }

    // @internal
    #[test]
    fn lc_messages_used_when_lc_all_unset() {
        with_env(
            &[
                ("LC_ALL", None),
                ("LC_MESSAGES", Some("es_ES.UTF-8")),
                ("LANG", Some("it_IT.UTF-8")),
            ],
            || {
                assert_eq!(detect_locale(), Locale::Spanish);
            },
        );
    }

    // @internal
    #[test]
    fn lang_used_when_no_lc_var_set() {
        with_env(
            &[
                ("LC_ALL", None),
                ("LC_MESSAGES", None),
                ("LANG", Some("it_CH.UTF-8")),
            ],
            || {
                assert_eq!(detect_locale(), Locale::Italian);
            },
        );
    }

    // @internal
    #[test]
    fn c_locale_falls_back_to_default() {
        with_env(
            &[("LC_ALL", Some("C")), ("LC_MESSAGES", None), ("LANG", None)],
            || {
                assert_eq!(detect_locale(), Locale::default());
            },
        );
    }

    // @internal
    #[test]
    fn posix_locale_falls_back_to_default() {
        with_env(
            &[
                ("LC_ALL", None),
                ("LC_MESSAGES", Some("POSIX")),
                ("LANG", None),
            ],
            || {
                assert_eq!(detect_locale(), Locale::default());
            },
        );
    }

    // @internal
    #[test]
    fn no_env_vars_returns_default() {
        with_env(
            &[("LC_ALL", None), ("LC_MESSAGES", None), ("LANG", None)],
            || {
                assert_eq!(detect_locale(), Locale::default());
            },
        );
    }

    // @internal
    #[test]
    fn unknown_language_falls_back_to_default() {
        with_env(
            &[
                ("LC_ALL", None),
                ("LC_MESSAGES", None),
                ("LANG", Some("zz_ZZ.UTF-8")),
            ],
            || {
                assert_eq!(detect_locale(), Locale::default());
            },
        );
    }
}
