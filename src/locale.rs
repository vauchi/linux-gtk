// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use vauchi_app::i18n::Locale;
use vauchi_app::ui::{AppEngine, RenderContext};

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
                && let Some(locale) = vauchi_app::i18n::translated_locale(code)
            {
                return locale;
            }
        }
    }
    Locale::default()
}

/// The environment's language tag, unparsed, for Core's `system_locale`:
/// Core alone maps it to a language (vauchi/private#721, ADR-066). Same
/// precedence and `C`/`POSIX` skipping as [`detect_locale`].
pub fn system_locale_from(value_for: impl Fn(&str) -> Option<String>) -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .filter_map(value_for)
        .find(|tag| {
            let language = tag.split(['_', '.', '@']).next().unwrap_or_default();
            !matches!(language, "" | "C" | "POSIX")
        })
}

pub fn report_system_locale(engine: &mut AppEngine, value_for: impl Fn(&str) -> Option<String>) {
    let ctx = RenderContext {
        system_locale: system_locale_from(value_for),
        ..engine.render_context().clone()
    };
    engine.set_render_context(ctx);
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
            Locale::GERMAN
        );
    }

    // @internal
    #[test]
    fn lc_messages_used_when_lc_all_unset() {
        assert_eq!(
            detect_test_locale(&[("LC_MESSAGES", "es_ES.UTF-8"), ("LANG", "it_IT.UTF-8")]),
            Locale::SPANISH
        );
    }

    // @internal
    #[test]
    fn lang_used_when_no_lc_var_set() {
        assert_eq!(
            detect_test_locale(&[("LANG", "it_CH.UTF-8")]),
            Locale::ITALIAN
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

    fn env<'a>(values: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            values
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_string())
        }
    }

    // @internal
    #[test]
    fn the_raw_tag_of_the_first_set_variable_is_reported() {
        assert_eq!(
            system_locale_from(env(&[("LC_ALL", "fr_CH.UTF-8"), ("LANG", "de_DE")])).as_deref(),
            Some("fr_CH.UTF-8")
        );
        assert_eq!(
            system_locale_from(env(&[("LC_ALL", "C.UTF-8"), ("LANG", "de_DE.UTF-8")])).as_deref(),
            Some("de_DE.UTF-8")
        );
        assert_eq!(
            system_locale_from(env(&[("LC_ALL", "POSIX"), ("LANG", "")])),
            None
        );
    }

    // @internal
    #[test]
    fn following_the_system_renders_the_environment_language() {
        let mut engine = vauchi_app::ui::AppEngine::new(
            vauchi_core::api::Vauchi::in_memory().expect("in-memory core"),
        );

        report_system_locale(&mut engine, env(&[("LANG", "de_DE.UTF-8")]));

        assert_eq!(
            engine.render_context().locale,
            None,
            "still following the system"
        );
        assert_eq!(engine.render_context().resolved_locale(), Locale::GERMAN);
    }
}
