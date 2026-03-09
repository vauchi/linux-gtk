// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Maps GTK widget signals to `UserAction` enum values.

use vauchi_core::ui::UserAction;

/// Handles mapping from GTK signals to core UserActions.
pub struct ActionHandler;

impl ActionHandler {
    pub fn new() -> Self {
        Self
    }
}
