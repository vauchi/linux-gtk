<!-- SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me> -->
<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

> **Mirror:** This repo is a read-only mirror of [gitlab.com/vauchi/linux-gtk](https://gitlab.com/vauchi/linux-gtk). Please open issues and merge requests there.

[![Pipeline](https://img.shields.io/endpoint?url=https://vauchi.gitlab.io/linux-gtk/badges/pipeline.json&label=pipeline)](https://gitlab.com/vauchi/linux-gtk/-/pipelines)
[![REUSE](https://api.reuse.software/badge/gitlab.com/vauchi/linux-gtk)](https://api.reuse.software/info/gitlab.com/vauchi/linux-gtk)

> [!WARNING]
> **Pre-Alpha Software** - This project is under heavy development
> and not ready for production use.
> APIs may change without notice. Use at your own risk.

# Vauchi Linux GTK

Native Linux desktop app for Vauchi — privacy-focused contact card exchange.

Built with GTK4-rs (Rust). Uses `vauchi-core` as a direct Rust dependency (no FFI overhead).

## Prerequisites

- GTK4 development libraries (`libgtk-4-dev`)
- libadwaita development libraries (`libadwaita-1-dev`)
- Rust 1.78+ (see `rust-toolchain.toml`)

## Build

```bash
cargo build
cargo test
```

## Architecture

This app implements the core-driven UI contract:

- **ScreenRenderer** renders `ScreenModel` from core (direct Rust types)
- **14 component renderers** map to core's `Component` enum variants using GTK4 widgets
- **ActionHandler** maps user input to `UserAction` enum
- **Platform chrome**: HeaderBar, GNotification, libadwaita styling

All business logic lives in `vauchi-core` (Rust). This repo is a pure rendering layer.

## Packaging

- Flatpak (Flathub)
- AppImage
- .deb

## License

GPL-3.0-or-later
