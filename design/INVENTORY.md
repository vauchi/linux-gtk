<!-- SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me> -->
<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# gVauchi (GTK) — Design Inventory

## Components (18 types)

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
| 11 | ActionList | `action_list.rs` | `gtk4::ListBox` of buttons | Yes (ActionPressed) | Settings, Backup, More |
| 12 | SettingsGroup | `settings_group.rs` | `adw::PreferencesGroup` with toggles/buttons | Yes (ActionPressed, ItemToggled) | Settings, Privacy |
| 13 | EditableText | `editable_text.rs` | `gtk4::Label` ↔ `gtk4::Entry` toggle | Yes (TextChanged) | MyInfo (name editing) |
| 14 | InlineConfirm | `inline_confirm.rs` | `gtk4::Box` with warning + confirm/cancel | Yes (ActionPressed) | EmergencyShred |
| 15 | Divider | `divider.rs` | `gtk4::Separator` (horizontal) | No | Various (visual separator) |
| 16 | Banner | `banner.rs` | `gtk4::Box` (horizontal, label + button) | Yes (ActionPressed) | Informational bar with optional action |
| 17 | AvatarPreview | `avatar_preview.rs` | `gtk4::Picture` / `gtk4::Label` (circle) | Yes (ActionPressed when editable) | Avatar Editor, MyInfo |
| 18 | Slider | `slider.rs` | `gtk4::Scale` | Yes (SliderChanged) | Avatar Editor (brightness) |

## Navigation

### Sidebar (5 top-level screens)

Built dynamically from `AppEngine::available_screens()`. When no identity exists, only Onboarding appears.

| # | Screen | Sidebar Label | Key Components |
|---|--------|--------------|----------------|
| 1 | My Info | i18n `nav.myCard` | CardPreview, FieldList, EditableText, ActionList |
| 2 | Contacts | i18n `nav.contacts` | ContactList |
| 3 | Exchange | i18n `nav.exchange` | QrCode, ToggleList, Text, StatusIndicator |
| 4 | Groups | i18n `nav.groups` | ToggleList, ActionList |
| 5 | More | i18n `nav.more` | ActionList (navigation hub) |

### More screen sub-navigation

The More screen renders an ActionList linking to secondary screens.
All are reached via `navigate_to()`.

| Target | Screen ID | Key Components |
|--------|-----------|----------------|
| Sync | `sync` | StatusIndicator, ActionList |
| Devices | `device_linking` | QrCode, ActionList, StatusIndicator |
| Settings | `settings` | SettingsGroup, ActionList |
| Backup | `backup` | ActionList, TextInput, InfoPanel |
| Privacy | `privacy` | SettingsGroup, ActionList |
| Help | `help` | InfoPanel, ActionList |

### Action-navigated sub-screens

Reached via ActionResult navigation (e.g., tapping a contact):

| Screen | Screen ID | Reached From |
|--------|-----------|--------------|
| Contact Detail | `contact_detail` | Contacts (select item) |
| Contact Edit | `contact_edit` | Contact Detail (edit action) |
| Contact Visibility | `contact_visibility` | Contact Detail (visibility action) |
| Entry Detail | `entry_detail` | MyInfo (tap field) |
| Group Detail | `group_detail` | Groups (select item) |
| Lock | `lock` | Settings (password set) |
| Duress PIN | `duress_pin` | Settings (action) |
| Emergency Shred | `emergency_shred` | Settings (action) |
| Delivery Status | `delivery_status` | Settings/More (action) |
| Recovery | `recovery` | Settings (action) |
| Support | `support` | Help (action) |
| Onboarding | `onboarding` | No identity exists |

## Workflows (7)

### W1: Onboarding

```text
[No Identity] → Onboarding screen
  → Enter name (TextInput)
  → Select groups (ToggleList)
  → Add fields (FieldList + TextInput)
  → Security explanation (InfoPanel)
  → Ready → MyInfo
```

### W2: Contact Exchange

```text
Exchange screen → Show QR (QrCode display)
  → Other party scans QR
  OR → Paste QR data (Entry + paste dialog)
  → Exchange complete → contact added
  → Contacts screen shows new entry
```

### W3: Contact Management

```text
Contacts → Select contact (ContactList)
  → Contact detail (CardPreview, FieldList)
  → Edit fields → Save
  → Delete contact (InlineConfirm)
```

### W4: Backup

```text
More → Backup
  → Export backup (password entry → file save)
  → Import backup (file select → password → restore)
```

### W5: Settings

```text
More → Settings → SettingsGroup items
  → Toggle delivery receipts, suppress presence
  → Change password (TextInput)
  → Manage devices (DeviceLinking)
  → Duress PIN (DuressPin)
  → Emergency wipe (EmergencyShred)
```

### W6: Device Linking

```text
More → Devices → Generate link QR (QrCode)
  → Second device scans QR
  → Exchange device keys
  → Linked device appears in list
```

### W7: Groups & Visibility

```text
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

**AT-SPI labels set on all interactive widgets.** Every component
uses `update_property(&[Property::Label(...)])` with descriptive
text. Key coverage:

- Navigation sidebar: `Property::Label("Navigation")`
- All list widgets: `Property::Label("Contacts")`, `Property::Label("Fields")`, `Property::Label("Actions")`
- Text inputs: `Property::Label(label)` + `Property::Placeholder(...)`
- QR code: `AccessibleRole::Img` + `Property::Label("QR code for contact exchange")`
- Settings toggles: per-item `Property::Label`
- Divider: `AccessibleRole::Separator`
- All other components: `Property::Label(title)` or `Property::Label(warning)`

AT-SPI tests in `tests/atspi/` verify the accessibility tree.

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Alt+1..5 | Navigate to sidebar screen (My Card, Contacts, Exchange, Groups, More) |
