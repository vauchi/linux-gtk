// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Platform initialization — creates Vauchi instance with Linux-specific config.

use vauchi_core::api::Vauchi;
use vauchi_core::network::MockTransport;

pub fn init_vauchi() -> Result<Vauchi<MockTransport>, Box<dyn std::error::Error>> {
    // For now, use in-memory storage. Real implementation will use XDG data dir.
    Ok(Vauchi::in_memory()?)
}
