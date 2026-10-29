// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! GTK's NFC reader executes Core's frames and reports the wire, nothing
//! more (vauchi/private#764): Core frames the APDUs and reads the status
//! words, the reader transmits verbatim on one card connection.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use vauchi_core::Event;
use vauchi_gtk::platform::nfc_reader::{Card, ReaderSession};

type Answer = Result<Vec<u8>, String>;

/// A card that records what it was sent and answers from a script.
#[derive(Clone, Default)]
struct ScriptedCard {
    sent: Arc<Mutex<Vec<Vec<u8>>>>,
    answers: Arc<Mutex<VecDeque<Answer>>>,
}

impl ScriptedCard {
    fn answering(answers: &[Result<&[u8], &str>]) -> Self {
        let card = Self::default();
        *card.answers.lock().unwrap() = answers
            .iter()
            .map(|answer| answer.map(<[u8]>::to_vec).map_err(str::to_string))
            .collect();
        card
    }

    fn sent(&self) -> Vec<Vec<u8>> {
        self.sent.lock().unwrap().clone()
    }
}

impl Card for ScriptedCard {
    fn transmit(&mut self, apdu: &[u8]) -> Result<Vec<u8>, String> {
        self.sent.lock().unwrap().push(apdu.to_vec());
        self.answers
            .lock()
            .unwrap()
            .pop_front()
            .expect("a scripted answer for every frame")
    }
}

fn received(bytes: &[u8]) -> Event {
    Event::NfcApduReceived {
        bytes: bytes.to_vec(),
    }
}

fn failed(reason: &str) -> Event {
    Event::NfcFailed {
        reason: reason.into(),
    }
}

const SELECT: &[u8] = &[0x00, 0xA4, 0x04, 0x00, 0x01, 0xF0];
const EXCHANGE: &[u8] = &[0x00, 0xE0, 0x00, 0x00, 0x01, 0x2A];
const CARD_SEND: &[u8] = &[0x00, 0xE2, 0x00, 0x00, 0x01, 0x07];

// @internal
#[test]
fn activation_transmits_cores_frames_verbatim_and_reports_every_response() {
    let card = ScriptedCard::answering(&[Ok(&[0x90, 0x00]), Ok(&[0x01, 0x02, 0x90, 0x00])]);
    let mut session = ReaderSession::new();

    let events = session.activate(|| Ok(card.clone()), &[SELECT.to_vec(), EXCHANGE.to_vec()]);

    assert_eq!(card.sent(), vec![SELECT.to_vec(), EXCHANGE.to_vec()]);
    assert_eq!(
        events,
        vec![received(&[0x90, 0x00]), received(&[0x01, 0x02, 0x90, 0x00])]
    );
}

// @internal
#[test]
fn a_later_send_uses_the_same_card_connection() {
    let card = ScriptedCard::answering(&[Ok(&[0x90, 0x00]), Ok(&[0x90, 0x00])]);
    let mut session = ReaderSession::new();
    let mut connections = 0;
    session.activate(
        || {
            connections += 1;
            Ok(card.clone())
        },
        &[SELECT.to_vec()],
    );

    let events = session.send(&[CARD_SEND.to_vec()]);

    assert_eq!(connections, 1);
    assert_eq!(card.sent(), vec![SELECT.to_vec(), CARD_SEND.to_vec()]);
    assert_eq!(events, vec![received(&[0x90, 0x00])]);
}

/// A failing status word is Core's to interpret: the reader reports it
/// like any other response and keeps transmitting.
// @internal
#[test]
fn a_failing_status_word_is_reported_not_interpreted() {
    let card = ScriptedCard::answering(&[Ok(&[0x6A, 0x82]), Ok(&[0x90, 0x00])]);
    let mut session = ReaderSession::new();

    let events = session.activate(|| Ok(card.clone()), &[SELECT.to_vec(), EXCHANGE.to_vec()]);

    assert_eq!(
        events,
        vec![received(&[0x6A, 0x82]), received(&[0x90, 0x00])]
    );
}

// @internal
#[test]
fn a_lost_tag_fails_once_and_ends_the_connection() {
    let card = ScriptedCard::answering(&[Err("Card was removed"), Ok(&[0x90, 0x00])]);
    let mut session = ReaderSession::new();

    let events = session.activate(|| Ok(card.clone()), &[SELECT.to_vec(), EXCHANGE.to_vec()]);
    let after = session.send(&[CARD_SEND.to_vec()]);

    assert_eq!(events, vec![failed("Card was removed")]);
    assert_eq!(card.sent(), vec![SELECT.to_vec()]);
    assert_eq!(after, vec![failed("no NFC tag is connected")]);
}

// @internal
#[test]
fn a_reader_that_cannot_connect_reports_why() {
    let mut session = ReaderSession::<ScriptedCard>::new();

    let events = session.activate(|| Err("No smart card reader".into()), &[SELECT.to_vec()]);

    assert_eq!(events, vec![failed("No smart card reader")]);
}

// @internal
#[test]
fn deactivation_ends_the_connection() {
    let card = ScriptedCard::answering(&[Ok(&[0x90, 0x00])]);
    let mut session = ReaderSession::new();
    session.activate(|| Ok(card.clone()), &[SELECT.to_vec()]);

    session.deactivate();
    let after = session.send(&[CARD_SEND.to_vec()]);

    assert_eq!(after, vec![failed("no NFC tag is connected")]);
    assert_eq!(card.sent(), vec![SELECT.to_vec()]);
}
