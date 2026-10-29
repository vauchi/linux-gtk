// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Executes Core's NFC reader commands on one card connection
//! (vauchi/private#764). Core frames the APDUs (`NfcActivate.apdus`,
//! `NfcSendApdu.apdus`) and reads the status words; the reader transmits
//! verbatim and reports every response as `Event::NfcApduReceived`.

use vauchi_core::Event;

/// One connected tag, as the reader hardware sees it.
pub trait Card {
    /// Sends one APDU and returns the raw response, status word included.
    fn transmit(&mut self, apdu: &[u8]) -> Result<Vec<u8>, String>;
}

pub struct ReaderSession<C> {
    card: Option<C>,
}

impl<C: Card> Default for ReaderSession<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Card> ReaderSession<C> {
    pub fn new() -> Self {
        Self { card: None }
    }

    /// `NfcActivate`: a fresh connection, then Core's frames.
    pub fn activate(
        &mut self,
        connect: impl FnOnce() -> Result<C, String>,
        apdus: &[Vec<u8>],
    ) -> Vec<Event> {
        self.card = None;
        match connect() {
            Ok(card) => {
                self.card = Some(card);
                self.send(apdus)
            }
            Err(reason) => vec![Event::NfcFailed { reason }],
        }
    }

    /// `NfcSendApdu`: more of Core's frames on the connection `activate`
    /// opened.
    pub fn send(&mut self, apdus: &[Vec<u8>]) -> Vec<Event> {
        let Some(card) = self.card.as_mut() else {
            return vec![Event::NfcFailed {
                reason: "no NFC tag is connected".into(),
            }];
        };
        let mut events = Vec::with_capacity(apdus.len());
        for apdu in apdus {
            match card.transmit(apdu) {
                Ok(bytes) => events.push(Event::NfcApduReceived { bytes }),
                Err(reason) => {
                    self.card = None;
                    events.push(Event::NfcFailed { reason });
                    break;
                }
            }
        }
        events
    }

    /// `NfcDeactivate`.
    pub fn deactivate(&mut self) {
        self.card = None;
    }
}
