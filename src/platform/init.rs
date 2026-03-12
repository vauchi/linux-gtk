// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Platform initialization — creates Vauchi instance with Linux-specific config.

use std::path::PathBuf;

use vauchi_core::api::{Vauchi, VauchiConfig};
use vauchi_core::network::MockTransport;

/// Returns the XDG data directory for vauchi (`$XDG_DATA_HOME/vauchi` or `~/.local/share/vauchi`).
fn data_dir() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").expect("HOME environment variable not set");
            PathBuf::from(home).join(".local/share")
        })
        .join("vauchi")
}

pub fn init_vauchi() -> Result<Vauchi<MockTransport>, Box<dyn std::error::Error>> {
    let data_path = data_dir();
    std::fs::create_dir_all(&data_path)?;

    let config = VauchiConfig::with_storage_path(&data_path);
    Ok(Vauchi::new(config)?)
}
