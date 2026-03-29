// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! NFC exchange handler via PC/SC (pcsclite).
//!
//! Implements NFC exchange for USB NFC readers (ACR122U, PN532) using
//! the PC/SC smartcard API. The desktop acts as a reader and exchanges
//! data with a phone doing NFC Host Card Emulation (HCE).
//!
//! APDU protocol:
//!   1. SELECT Vauchi AID (F0564155434849)
//!   2. EXCHANGE (INS 0xE0) — send our payload, receive theirs
//!
//! All PC/SC operations run on a background thread to avoid blocking
//! the GTK main loop.

#[cfg(feature = "nfc")]
mod inner {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;

    use gtk4::glib;
    use libadwaita as adw;

    use vauchi_app::i18n::{self, Locale};
    use vauchi_app::ui::AppEngine;
    use vauchi_core::exchange::ExchangeHardwareEvent;

    use crate::core_ui::screen_renderer::handle_app_engine_result;

    /// Vauchi NFC Application ID.
    const VAUCHI_AID: &[u8] = b"\xF0\x56\x41\x55\x43\x48\x49";

    /// APDU instruction for payload exchange.
    const INS_EXCHANGE: u8 = 0xE0;

    /// Build a SELECT AID APDU: 00 A4 04 00 <len> <AID>
    fn build_select_apdu() -> Vec<u8> {
        let mut apdu = vec![0x00, 0xA4, 0x04, 0x00, VAUCHI_AID.len() as u8];
        apdu.extend_from_slice(VAUCHI_AID);
        apdu
    }

    /// Build an EXCHANGE APDU: 80 E0 00 00 <Lc> <payload> 00
    fn build_exchange_apdu(payload: &[u8]) -> Vec<u8> {
        let mut apdu = vec![0x80, INS_EXCHANGE, 0x00, 0x00, payload.len() as u8];
        apdu.extend_from_slice(payload);
        apdu.push(0x00); // Le: max response
        apdu
    }

    /// Check APDU response status (last 2 bytes = 90 00).
    fn is_apdu_success(response: &[u8]) -> bool {
        response.len() >= 2
            && response[response.len() - 2] == 0x90
            && response[response.len() - 1] == 0x00
    }

    /// Activate NFC: poll for cards and exchange payload on tap.
    pub fn activate(
        container: &gtk4::Box,
        app_engine: &Rc<RefCell<AppEngine>>,
        toast_overlay: &adw::ToastOverlay,
        payload: Vec<u8>,
    ) {
        let container = container.clone();
        let app_engine = app_engine.clone();
        let toast_overlay = toast_overlay.clone();
        let (tx, rx) = mpsc::channel::<Result<Vec<u8>, String>>();

        let msg = i18n::get_string(Locale::default(), "platform.nfc_waiting");
        let toast = adw::Toast::new(&msg);
        toast.set_timeout(5);
        toast_overlay.add_toast(toast);

        std::thread::spawn(move || {
            let result = poll_and_exchange(&payload);
            tx.send(result).ok();
        });

        glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
            match rx.try_recv() {
                Ok(Ok(data)) => {
                    let event = ExchangeHardwareEvent::NfcDataReceived { data };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                    }
                    glib::ControlFlow::Break
                }
                Ok(Err(e)) => {
                    let event = ExchangeHardwareEvent::HardwareError {
                        transport: "NFC".into(),
                        error: e.clone(),
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                    }
                    let msg = i18n::get_string_with_args(
                        Locale::default(),
                        "platform.nfc_exchange_failed",
                        &[("error", &e)],
                    );
                    let toast = adw::Toast::new(&msg);
                    toast_overlay.add_toast(toast);
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
            }
        });
    }

    /// Poll PC/SC readers for a card and exchange data.
    fn poll_and_exchange(our_payload: &[u8]) -> Result<Vec<u8>, String> {
        let ctx =
            pcsc::Context::establish(pcsc::Scope::System).map_err(|e| format!("PC/SC: {}", e))?;

        // Get reader list
        let readers_buf_len = ctx
            .list_readers_len()
            .map_err(|e| format!("List readers: {}", e))?;
        let mut readers_buf = vec![0u8; readers_buf_len];
        let readers = ctx
            .list_readers(&mut readers_buf)
            .map_err(|e| format!("List readers: {}", e))?;

        // Try connecting to each reader
        for reader in readers {
            let card = match ctx.connect(reader, pcsc::ShareMode::Shared, pcsc::Protocols::ANY) {
                Ok(card) => card,
                Err(_) => continue, // No card present in this reader
            };

            // Step 1: SELECT Vauchi AID
            let select_apdu = build_select_apdu();
            let mut recv_buf = [0u8; 512];
            let response = card
                .transmit(&select_apdu, &mut recv_buf)
                .map_err(|e| format!("SELECT transmit: {}", e))?;

            if !is_apdu_success(response) {
                continue; // Not a Vauchi card
            }

            // Step 2: EXCHANGE — send our payload, receive theirs
            let exchange_apdu = build_exchange_apdu(our_payload);
            let response = card
                .transmit(&exchange_apdu, &mut recv_buf)
                .map_err(|e| format!("EXCHANGE transmit: {}", e))?;

            if is_apdu_success(response) && response.len() > 2 {
                // Strip SW1 SW2 (last 2 bytes)
                let data = response[..response.len() - 2].to_vec();
                return Ok(data);
            }
        }

        Err("No NFC device with Vauchi AID found".into())
    }
}

#[cfg(feature = "nfc")]
pub use inner::*;
