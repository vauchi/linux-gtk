// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! NFC reader over PC/SC (pcsclite) for USB readers (ACR122U, PN532): the
//! desktop reads a phone doing Host Card Emulation.
//!
//! Core frames every APDU and reads every status word (vauchi/private#764);
//! `nfc_reader::ReaderSession` executes its commands on one card. Here a
//! worker thread owns that session and the PC/SC calls, which block, and
//! the GTK main loop drains its observations to Core.

#[cfg(feature = "nfc")]
mod inner {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
    use std::time::Duration;

    use gtk4::glib;
    use libadwaita as adw;

    use vauchi_app::ui::AppEngine;
    use vauchi_core::Event;

    use crate::core_ui::contextual_surface::dispatch_platform_event;
    use crate::platform::nfc_reader::{Card, ReaderSession};

    /// PC/SC offers no tag-arrival callback through this API, so the wait
    /// for a tag polls; the interval only bounds how late a deactivation
    /// is noticed.
    const TAG_POLL: Duration = Duration::from_millis(200);

    enum Request {
        Activate(Vec<Vec<u8>>),
        Send(Vec<Vec<u8>>),
        Deactivate,
    }

    struct Worker {
        requests: Sender<Request>,
        /// Cleared by `deactivate`, so a wait for a tag ends without
        /// reporting a failure Core no longer expects.
        active: Arc<AtomicBool>,
    }

    static WORKER: Mutex<Option<Worker>> = Mutex::new(None);

    fn lock_worker() -> MutexGuard<'static, Option<Worker>> {
        WORKER.lock().unwrap_or_else(PoisonError::into_inner)
    }

    struct PcscCard(pcsc::Card);

    impl Card for PcscCard {
        fn transmit(&mut self, apdu: &[u8]) -> Result<Vec<u8>, String> {
            let mut response = vec![0u8; pcsc::MAX_BUFFER_SIZE_EXTENDED];
            self.0
                .transmit(apdu, &mut response)
                .map(<[u8]>::to_vec)
                .map_err(|e| e.to_string())
        }
    }

    /// `NfcActivate`: wait for a tag, then transmit Core's frames.
    pub fn activate(
        container: &gtk4::Box,
        app_engine: &Rc<RefCell<AppEngine>>,
        toast_overlay: &adw::ToastOverlay,
        apdus: Vec<Vec<u8>>,
    ) {
        let mut worker = lock_worker();
        let worker = worker.get_or_insert_with(|| start(container, app_engine, toast_overlay));
        worker.active.store(true, Ordering::SeqCst);
        let _ = worker.requests.send(Request::Activate(apdus));
    }

    /// `NfcSendApdu`: transmit Core's frames on the connected tag.
    pub fn send(apdus: Vec<Vec<u8>>) {
        if let Some(worker) = lock_worker().as_ref() {
            let _ = worker.requests.send(Request::Send(apdus));
        }
    }

    /// `NfcDeactivate`.
    pub fn deactivate() {
        if let Some(worker) = lock_worker().as_ref() {
            worker.active.store(false, Ordering::SeqCst);
            let _ = worker.requests.send(Request::Deactivate);
        }
    }

    fn start(
        container: &gtk4::Box,
        app_engine: &Rc<RefCell<AppEngine>>,
        toast_overlay: &adw::ToastOverlay,
    ) -> Worker {
        let (requests, incoming) = mpsc::channel();
        let (observations, observed) = mpsc::channel();
        let active = Arc::new(AtomicBool::new(false));
        let worker_active = active.clone();
        std::thread::Builder::new()
            .name("vauchi-nfc-reader".into())
            .spawn(move || serve(&incoming, &observations, &worker_active))
            .expect("spawn the NFC reader thread");

        let container = container.clone();
        let app_engine = app_engine.clone();
        let toast_overlay = toast_overlay.clone();
        glib::timeout_add_local(Duration::from_millis(100), move || {
            loop {
                match observed.try_recv() {
                    Ok(event) => {
                        dispatch_platform_event(&container, &app_engine, &toast_overlay, event)
                    }
                    Err(mpsc::TryRecvError::Empty) => break glib::ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => break glib::ControlFlow::Break,
                }
            }
        });
        Worker { requests, active }
    }

    fn serve(incoming: &Receiver<Request>, observations: &Sender<Event>, active: &AtomicBool) {
        let mut session = ReaderSession::<PcscCard>::new();
        for request in incoming {
            let events = match request {
                Request::Activate(apdus) => session.activate(|| wait_for_tag(active), &apdus),
                Request::Send(apdus) => session.send(&apdus),
                Request::Deactivate => {
                    session.deactivate();
                    Vec::new()
                }
            };
            if !active.load(Ordering::SeqCst) {
                continue;
            }
            for event in events {
                let _ = observations.send(event);
            }
        }
    }

    fn wait_for_tag(active: &AtomicBool) -> Result<PcscCard, String> {
        let context = pcsc::Context::establish(pcsc::Scope::System).map_err(|e| e.to_string())?;
        while active.load(Ordering::SeqCst) {
            let mut names = vec![0u8; context.list_readers_len().map_err(|e| e.to_string())?];
            for reader in context
                .list_readers(&mut names)
                .map_err(|e| e.to_string())?
            {
                if let Ok(card) =
                    context.connect(reader, pcsc::ShareMode::Shared, pcsc::Protocols::ANY)
                {
                    return Ok(PcscCard(card));
                }
            }
            std::thread::sleep(TAG_POLL);
        }
        Err("NFC reader deactivated".into())
    }
}

#[cfg(feature = "nfc")]
pub use inner::*;
