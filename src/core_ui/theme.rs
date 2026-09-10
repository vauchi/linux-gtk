// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Theme integration: maps core `Theme` colors to GTK4 CSS.
//!
//! Loads the default theme from `vauchi-app` and applies it as a CSS
//! stylesheet via `CssProvider`. The CSS is regenerated whenever the
//! theme changes, enabling runtime switching.

use gtk4::CssProvider;
use gtk4::gdk::Display;
use vauchi_app::theme::{FontFamilyTokens, Theme, ThemeColors};

/// Apply a `Theme` to the default GTK4 display via a `CssProvider`.
///
/// Replaces any previously applied vauchi theme CSS. Safe to call
/// multiple times for runtime theme switching.
pub fn apply_theme(theme: &Theme) {
    let css = generate_css(&theme.colors, &theme.tokens.font_family);
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

/// Generate a GTK4 CSS string from core `ThemeColors` and `FontFamilyTokens`.
///
/// Uses CSS custom properties (`--vauchi-*`) so components can
/// reference them, plus direct widget selectors for immediate effect.
fn generate_css(colors: &ThemeColors, _fonts: &FontFamilyTokens) -> String {
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
@define-color vauchi_focus_ring {focus_ring};

window {{
    background-color: @vauchi_bg_primary;
    color: @vauchi_text_primary;
}}

.suggested-action {{
    background-color: @vauchi_accent;
    color: @vauchi_bg_primary;
}}

.destructive-action {{
    background-color: @vauchi_error;
    color: @vauchi_bg_primary;
}}

/* `ActionTone::Serious` (verify a fingerprint, schedule a deletion, start
   recovery) is consequential but reversible or protective: an outline, not
   a fill, so it reads as neither the safe default nor "this destroys
   something". */
.serious-action {{
    background-color: transparent;
    color: @vauchi_warning;
    border: 2px solid @vauchi_warning;
}}

/* GTK's own `:focus-visible` pseudo-class carries keyboard focus; nothing
   here answered it before, so a keyboard user had no way to see which
   control had focus. `checkbutton` covers Toggle nodes; `.vauchi-row`
   covers the row containers built in `generic_surface/collections.rs`,
   which are plain boxes with no built-in focus styling of their own. */
button:focus-visible,
checkbutton:focus-visible,
.vauchi-row:focus-visible {{
    outline: 3px solid @vauchi_focus_ring;
    outline-offset: 2px;
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

/* Initials stand in for a missing avatar. Without a ground to sit on they
   read as a stray letter rather than as a person, so the class carries the
   whole shape: a fixed square, a fill and a radius. `AdwAvatar` styling
   does not reach a plain Label, which is why this cannot be borrowed. */
.avatar {{
    min-width: 96px;
    min-height: 96px;
    border-radius: 48px;
    background-color: @vauchi_bg_tertiary;
    color: @vauchi_text_primary;
    font-size: 1.6em;
    font-weight: bold;
}}

/* A row's avatar is the same idea at list scale. */
row .avatar,
list .avatar {{
    min-width: 32px;
    min-height: 32px;
    border-radius: 16px;
    font-size: 1em;
}}

/* Core asks for a natural-shaped image to keep its corners. */
.avatar.natural {{
    border-radius: 8px;
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
        focus_ring = colors.focus_ring.as_deref().unwrap_or(&colors.accent),
    )
}

// INLINE_TEST_REQUIRED: tests exercise private `generate_css` function
#[cfg(test)]
mod tests {
    use super::*;
    use vauchi_app::theme::{ThemeColors, default_theme};

    /// Both the standalone `Image` node and a list row's avatar hand their
    /// initials to a `Label` carrying the `avatar` class, but nothing ever
    /// defined that class — libadwaita's `avatar` styling belongs to
    /// `AdwAvatar`, not to any widget that borrows the name. So the class
    /// was inert and the initials rendered as a bare letter on the window
    /// background, which is the same defect `ios!651` fixed on Apple.
    // @internal
    #[test]
    fn generated_css_gives_the_avatar_class_a_body() {
        let css = generate_css(&default_theme().colors, &default_theme().tokens.font_family);

        assert!(
            css.contains(".avatar"),
            "no `.avatar` rule: initials render as a bare letter, since a \
             plain Label does not inherit AdwAvatar styling"
        );
        assert!(
            css.contains("border-radius"),
            "`.avatar` must round its ground, or the initials sit in a square"
        );
    }

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
            ..default_theme().colors
        };

        let css = generate_css(&colors, &default_theme().tokens.font_family);

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
        let css = generate_css(&theme.colors, &theme.tokens.font_family);

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
        let css = generate_css(&theme.colors, &theme.tokens.font_family);

        assert!(css.contains("window {"), "CSS should style the window");
        assert!(
            css.contains(".suggested-action {"),
            "CSS should style suggested actions"
        );
        assert!(
            css.contains(".destructive-action {"),
            "CSS should style destructive actions"
        );
    }

    /// `ActionTone::Serious` names a consequential-but-reversible action
    /// (verify a fingerprint, schedule a deletion, start recovery). It must
    /// read as neither the filled `.suggested-action` (the safe default) nor
    /// `.destructive-action` (irreversible), so the rule outlines in the
    /// warning colour and never fills with it.
    // @internal
    #[test]
    fn generate_css_styles_serious_action_as_a_warning_outline_with_no_fill() {
        let css = generate_css(&default_theme().colors);

        assert!(
            css.contains(".serious-action {"),
            "CSS should style serious actions"
        );
        assert!(
            !css.contains("background-color: @vauchi_warning"),
            "`.serious-action` must outline, not fill, or it reads as destructive"
        );
    }

    /// GTK draws keyboard focus through the `:focus-visible` pseudo-class;
    /// nothing in this stylesheet answered it before, so a keyboard user saw
    /// no indication of which button or row currently had focus.
    // @internal
    #[test]
    fn generate_css_draws_a_focus_ring_from_the_theme() {
        let css = generate_css(&default_theme().colors);

        assert!(
            css.contains(":focus-visible"),
            "CSS should draw a ring on keyboard focus"
        );
        assert!(
            css.contains("outline: 3px solid"),
            "focus ring should be a 3px outline"
        );
        assert!(
            css.contains("outline-offset: 2px"),
            "focus ring should sit clear of the widget edge"
        );
    }

    /// The theme's dedicated `focus-ring` role (ADR-038 Amendment 3) wins
    /// when a theme supplies one — falling back to `accent` is only for
    /// themes predating that amendment.
    // @internal
    #[test]
    fn generate_css_focus_ring_prefers_the_dedicated_theme_role_over_accent() {
        let colors = ThemeColors {
            accent: "#0000ff".to_string(),
            focus_ring: Some("#ff00ff".to_string()),
            ..default_theme().colors
        };

        let css = generate_css(&colors);

        assert!(
            css.contains("@define-color vauchi_focus_ring #ff00ff;"),
            "focus ring should use the theme's dedicated focus-ring colour"
        );
    }

    /// Themes from before ADR-038 Amendment 3 have no `focus-ring` role at
    /// all; the ring must still draw, using `accent` so it stays visible.
    // @internal
    #[test]
    fn generate_css_focus_ring_falls_back_to_accent_when_theme_has_none() {
        let colors = ThemeColors {
            accent: "#0000ff".to_string(),
            focus_ring: None,
            ..default_theme().colors
        };

        let css = generate_css(&colors);

        assert!(
            css.contains("@define-color vauchi_focus_ring #0000ff;"),
            "focus ring should fall back to accent when the theme has no focus-ring role"
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
            ..default_theme().colors
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
            ..default_theme().colors
        };

        let dark_css = generate_css(&dark, &default_theme().tokens.font_family);
        let light_css = generate_css(&light, &default_theme().tokens.font_family);

        assert_ne!(
            dark_css, light_css,
            "Different themes should produce different CSS"
        );
    }

    /// `PresentationTextStyle::Heading` maps to the `title-2` GTK style
    /// class (`generic_surface/controls.rs`) and `::Monospace` maps to
    /// GTK's built-in `monospace` class — the CSS must give both a brand
    /// typeface, and the base `window` rule carries the body family for
    /// everything that inherits from it.
    #[test]
    fn generate_css_sets_font_families_from_design_tokens() {
        let fonts = default_theme().tokens.font_family;
        let css = generate_css(&default_theme().colors, &fonts);

        assert!(
            css.contains(r#"font-family: "Hanken Grotesk""#),
            "window rule should set the body family for base widgets"
        );
        assert!(
            css.contains(r#"font-family: "Bricolage Grotesque""#),
            "`.title-2` rule should set the display family for headings"
        );
        assert!(
            css.contains("font-weight: 700"),
            "`.title-2` rule should use the display weight for headings"
        );
        assert!(
            css.contains(r#"font-family: "JetBrains Mono""#),
            "`.monospace` rule should set the mono family"
        );
    }
}
