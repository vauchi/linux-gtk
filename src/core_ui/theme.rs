// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Theme integration: maps core `Theme` colors to GTK4 CSS.
//!
//! Loads the default theme from `vauchi-app` and applies it as a CSS
//! stylesheet via `CssProvider`. The CSS is regenerated whenever the
//! theme changes, enabling runtime switching.

use gtk4::CssProvider;
use gtk4::gdk::Display;
use vauchi_app::theme::{Theme, ThemeColors};

/// Apply a `Theme` to the default GTK4 display via a `CssProvider`.
///
/// Replaces any previously applied vauchi theme CSS. Safe to call
/// multiple times for runtime theme switching.
pub fn apply_theme(theme: &Theme) {
    let css = generate_css(&theme.colors);
    let provider = CssProvider::new();
    provider.load_from_data(&css);

    if let Some(display) = Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

/// Apply the default bundled theme.
pub fn apply_default_theme() {
    let theme = vauchi_app::theme::default_theme();
    apply_theme(&theme);
}

/// Generate a GTK4 CSS string from core `ThemeColors`.
///
/// Uses CSS custom properties (`--vauchi-*`) so components can
/// reference them, plus direct widget selectors for immediate effect.
fn generate_css(colors: &ThemeColors) -> String {
    format!(
        r#"
/* Vauchi core theme — auto-generated from vauchi-app::theme */
@define-color vauchi_bg_primary {bg_primary};
@define-color vauchi_bg_secondary {bg_secondary};
@define-color vauchi_bg_tertiary {bg_tertiary};
@define-color vauchi_text_primary {text_primary};
@define-color vauchi_text_secondary {text_secondary};
@define-color vauchi_accent {accent};
@define-color vauchi_accent_dark {accent_dark};
@define-color vauchi_success {success};
@define-color vauchi_error {error};
@define-color vauchi_warning {warning};
@define-color vauchi_border {border};

window {{
    background-color: @vauchi_bg_primary;
    color: @vauchi_text_primary;
}}

.navigation-sidebar {{
    background-color: @vauchi_bg_secondary;
    border-right: 1px solid @vauchi_border;
}}

.navigation-sidebar row:selected {{
    background-color: @vauchi_bg_tertiary;
}}

.suggested-action {{
    background-color: @vauchi_accent;
    color: @vauchi_bg_primary;
}}

.destructive-action {{
    background-color: @vauchi_error;
    color: @vauchi_bg_primary;
}}

.dim-label {{
    color: @vauchi_text_secondary;
}}

.error {{
    color: @vauchi_error;
}}

.card {{
    border-color: @vauchi_border;
    background-color: @vauchi_bg_secondary;
}}

entry {{
    border-color: @vauchi_border;
}}
"#,
        bg_primary = colors.bg_primary,
        bg_secondary = colors.bg_secondary,
        bg_tertiary = colors.bg_tertiary,
        text_primary = colors.text_primary,
        text_secondary = colors.text_secondary,
        accent = colors.accent,
        accent_dark = colors.accent_dark,
        success = colors.success,
        error = colors.error,
        warning = colors.warning,
        border = colors.border,
    )
}

// INLINE_TEST_REQUIRED: tests exercise private `generate_css` function
#[cfg(test)]
mod tests {
    use super::*;
    use vauchi_app::theme::{ThemeColors, default_theme};

    #[test]
    fn generate_css_includes_all_colors() {
        let colors = ThemeColors {
            bg_primary: "#1e1e2e".to_string(),
            bg_secondary: "#181825".to_string(),
            bg_tertiary: "#313244".to_string(),
            text_primary: "#cdd6f4".to_string(),
            text_secondary: "#a6adc8".to_string(),
            accent: "#89b4fa".to_string(),
            accent_dark: "#74c7ec".to_string(),
            success: "#a6e3a1".to_string(),
            error: "#f38ba8".to_string(),
            warning: "#fab387".to_string(),
            border: "#45475a".to_string(),
        };

        let css = generate_css(&colors);

        assert!(
            css.contains("#1e1e2e"),
            "CSS should contain bg_primary color"
        );
        assert!(
            css.contains("#181825"),
            "CSS should contain bg_secondary color"
        );
        assert!(
            css.contains("#313244"),
            "CSS should contain bg_tertiary color"
        );
        assert!(
            css.contains("#cdd6f4"),
            "CSS should contain text_primary color"
        );
        assert!(
            css.contains("#a6adc8"),
            "CSS should contain text_secondary color"
        );
        assert!(css.contains("#89b4fa"), "CSS should contain accent color");
        assert!(
            css.contains("#74c7ec"),
            "CSS should contain accent_dark color"
        );
        assert!(css.contains("#a6e3a1"), "CSS should contain success color");
        assert!(css.contains("#f38ba8"), "CSS should contain error color");
        assert!(css.contains("#fab387"), "CSS should contain warning color");
        assert!(css.contains("#45475a"), "CSS should contain border color");
    }

    #[test]
    fn generate_css_has_define_color_directives() {
        let theme = default_theme();
        let css = generate_css(&theme.colors);

        assert!(
            css.contains("@define-color vauchi_bg_primary"),
            "CSS should define vauchi_bg_primary"
        );
        assert!(
            css.contains("@define-color vauchi_accent"),
            "CSS should define vauchi_accent"
        );
        assert!(
            css.contains("@define-color vauchi_error"),
            "CSS should define vauchi_error"
        );
    }

    #[test]
    fn generate_css_has_widget_selectors() {
        let theme = default_theme();
        let css = generate_css(&theme.colors);

        assert!(css.contains("window {"), "CSS should style the window");
        assert!(
            css.contains(".navigation-sidebar {"),
            "CSS should style the sidebar"
        );
        assert!(
            css.contains(".suggested-action {"),
            "CSS should style suggested actions"
        );
        assert!(
            css.contains(".destructive-action {"),
            "CSS should style destructive actions"
        );
    }

    #[test]
    fn generate_css_different_themes_produce_different_output() {
        let dark = ThemeColors {
            bg_primary: "#000000".to_string(),
            bg_secondary: "#111111".to_string(),
            bg_tertiary: "#222222".to_string(),
            text_primary: "#ffffff".to_string(),
            text_secondary: "#cccccc".to_string(),
            accent: "#0000ff".to_string(),
            accent_dark: "#000099".to_string(),
            success: "#00ff00".to_string(),
            error: "#ff0000".to_string(),
            warning: "#ffff00".to_string(),
            border: "#333333".to_string(),
        };
        let light = ThemeColors {
            bg_primary: "#ffffff".to_string(),
            bg_secondary: "#eeeeee".to_string(),
            bg_tertiary: "#dddddd".to_string(),
            text_primary: "#000000".to_string(),
            text_secondary: "#333333".to_string(),
            accent: "#0066cc".to_string(),
            accent_dark: "#004488".to_string(),
            success: "#228b22".to_string(),
            error: "#cc0000".to_string(),
            warning: "#cc8800".to_string(),
            border: "#cccccc".to_string(),
        };

        let dark_css = generate_css(&dark);
        let light_css = generate_css(&light);

        assert_ne!(
            dark_css, light_css,
            "Different themes should produce different CSS"
        );
    }
}
