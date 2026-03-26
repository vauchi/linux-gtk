<!-- SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me> -->
<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# gVauchi (GTK) — Design Inventory

## Components (16 types)

All components are rendered by `src/core_ui/components/mod.rs` via `render_component()`.
Each component maps a vauchi-core `Component` enum variant to GTK4/libadwaita widgets.

| # | Component | File | GTK Widget(s) | Interactive | Used On |
|---|-----------|------|---------------|-------------|---------|
| 1 | Text | `text.rs` | `gtk4::Label` | No | All screens (titles, descriptions) |
| 2 | TextInput | `text_input.rs` | `gtk4::Entry` | Yes (TextChanged on Enter/focus-leave) | Onboarding, Settings |
| 3 | PinInput | `pin_input.rs` | `gtk4::Entry` (password mode) | Yes (TextChanged, auto-advance) | Lock, DuressPin |
| 4 | ToggleList | `toggle_list.rs` | `gtk4::CheckButton` (multiple) | Yes (ItemToggled) | Onboarding (groups), Exchange (group preselect) |
| 5 | ContactList | `contact_list.rs` | `gtk4::ListBox` + search Entry | Yes (ListItemSelected) | Contacts |
| 6 | FieldList | `field_list.rs` | `gtk4::ListBox` with group headers | Yes (ListItemSelected) | MyInfo, ContactDetail |
| 7 | CardPreview | `card_preview.rs` | `gtk4::Frame` with group tabs | Yes (GroupViewSelected) | MyInfo, ContactDetail |
| 8 | QrCode | `qr_code.rs` | `gtk4::DrawingArea` (cairo) or Entry+Button | Yes (scan/paste) | Exchange |
| 9 | InfoPanel | `info_panel.rs` | `gtk4::Box` with icon + title + items | No | Help, Support, Recovery |
| 10 | StatusIndicator | `status_indicator.rs` | `gtk4::Box` with icon + label | No | DeliveryStatus, Sync |
| 11 | ActionList | `action_list.rs` | `gtk4::ListBox` of buttons | Yes (ActionPressed) | Settings, Backup, DeviceLinking |
| 12 | SettingsGroup | `settings_group.rs` | `adw::PreferencesGroup` with toggles/buttons | Yes (ActionPressed, ItemToggled) | Settings, Privacy |
| 13 | EditableText | `editable_text.rs` | `gtk4::Label` ↔ `gtk4::Entry` toggle | Yes (TextChanged) | MyInfo (name editing) |
| 14 | InlineConfirm | `inline_confirm.rs` | `gtk4::Box` with warning + confirm/cancel | Yes (ActionPressed) | EmergencyShred |
| 15 | Divider | `divider.rs` | `gtk4::Separator` (horizontal) | No | Various (visual separator) |
| 16 | Banner | `banner.rs` | `gtk4::Box` (horizontal, label + button) | Yes (ActionPressed) | Informational bar with optional action |

## Screens (17 + catch-all)

Navigation via sidebar `gtk4::ListBox`. Screen rendering through `AppEngine::navigate_to()` → `ScreenModel` → `render_screen_model()`.

| # | Screen | Sidebar Label | Key Components | Entry Conditions |
|---|--------|--------------|----------------|------------------|
| 1 | Onboarding | "Setup" | Text, TextInput, ToggleList, FieldList | No identity exists |
| 2 | MyInfo | "My Info" | CardPreview, FieldList, EditableText, ActionList | Default if identity exists |
| 3 | Contacts | "Contacts" | ContactList | Always available |
| 4 | Exchange | "Exchange" | QrCode, ToggleList, Text, StatusIndicator | Always available |
| 5 | Settings | "Settings" | SettingsGroup, ActionList | Always available |
| 6 | Help | "Help" | InfoPanel, ActionList | Always available |
| 7 | Backup | "Backup" | ActionList, TextInput, InfoPanel | Always available |
| 8 | Lock | "Lock" | PinInput, Text | Password set |
| 9 | DeviceLinking | "Device Linking" | QrCode, ActionList, StatusIndicator | Always available |
| 10 | DuressPin | "Duress PIN" | PinInput, Text | Always available |
| 11 | EmergencyShred | "Emergency Shred" | InlineConfirm, Text | Always available |
| 12 | DeliveryStatus | "Delivery Status" | StatusIndicator, ContactList | Always available |
| 13 | Sync | "Sync" | StatusIndicator, ActionList | Always available |
| 14 | Recovery | "Recovery" | InfoPanel, ActionList | Always available |
| 15 | Groups | "Groups" | ToggleList, ActionList | Always available |
| 16 | Privacy | "Privacy" | SettingsGroup, ActionList | Always available |
| 17 | Support | "Support" | InfoPanel, ActionList | Always available |

## Workflows (7)

### W1: Onboarding
```
[No Identity] → Onboarding screen
  → Enter name (TextInput)
  → Select groups (ToggleList)
  → Add fields (FieldList + TextInput)
  → Security explanation (InfoPanel)
  → Ready → MyInfo
```

### W2: Contact Exchange
```
Exchange screen → Show QR (QrCode display)
  → Other party scans QR
  OR → Paste QR data (Entry + paste dialog)
  → Exchange complete → contact added
  → Contacts screen shows new entry
```

### W3: Contact Management
```
Contacts → Select contact (ContactList)
  → Contact detail (CardPreview, FieldList)
  → Edit fields → Save
  → Delete contact (InlineConfirm)
```

### W4: Backup
```
Settings → Backup
  → Export backup (password entry → file save)
  → Import backup (file select → password → restore)
```

### W5: Settings
```
Settings → SettingsGroup items
  → Toggle delivery receipts, suppress presence
  → Change password (TextInput)
  → Manage devices (DeviceLinking)
  → Duress PIN (DuressPin)
  → Emergency wipe (EmergencyShred)
```

### W6: Device Linking
```
DeviceLinking → Generate link QR (QrCode)
  → Second device scans QR
  → Exchange device keys
  → Linked device appears in list
```

### W7: Groups & Visibility
```
Groups → View groups list (ToggleList)
  → Create group (TextInput)
  → Assign contacts to groups
  → Set field visibility per group
  → CardPreview shows group-filtered view
```

## Hardware Integration

| Hardware | Module | Feature Flag | Detection | Status |
|----------|--------|-------------|-----------|--------|
| Camera | `platform/camera.rs` | `camera` (optional) | `/dev/video*` | QR scanning via nokhwa + rqrr |
| BLE | `platform/ble.rs` | `ble` (default) | `/sys/class/bluetooth/` | BlueZ via bluer + tokio |
| Audio | `platform/audio.rs` | `audio` (default) | `/proc/asound/cards` | Ultrasonic via CPAL |
| NFC | `platform/nfc.rs` | `nfc` (optional) | `/dev/nfc*` | PC/SC exchange via pcsclite (SELECT AID + EXCHANGE APDU) |

## Accessibility Status

**Current: AT-SPI labels set on all interactive widgets.** Every component uses `update_property(&[Property::Label(...)])` with descriptive text. Key coverage:

- Navigation sidebar: `Property::Label("Navigation")`
- All list widgets: `Property::Label("Contacts")`, `Property::Label("Fields")`, `Property::Label("Actions")`
- Text inputs: `Property::Label(label)` + `Property::Placeholder(...)`
- QR code: `AccessibleRole::Img` + `Property::Label("QR code for contact exchange")`
- Settings toggles: per-item `Property::Label`
- Divider: `AccessibleRole::Separator`
- All other components: `Property::Label(title)` or `Property::Label(warning)`

AT-SPI tests in `tests/atspi/` verify the accessibility tree.
