// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! BLE exchange handler via BlueZ (bluer).
//!
//! Implements scanning, advertising, connecting, and GATT operations
//! for the vauchi BLE exchange protocol. All BLE operations run on a
//! background thread with a tokio runtime, reporting results back to
//! the GTK main loop via mpsc channels.

#[cfg(feature = "ble")]
mod inner {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;

    use gtk4::glib;
    use libadwaita as adw;

    use vauchi_core::exchange::ExchangeHardwareEvent;
    use vauchi_core::ui::AppEngine;

    use crate::core_ui::screen_renderer::handle_app_engine_result;

    /// Start scanning for vauchi BLE peripherals.
    ///
    /// Discovers nearby devices advertising the vauchi service UUID.
    /// Reports each discovery as `BleDeviceDiscovered` to AppEngine.
    pub fn start_scanning(
        container: &gtk4::Box,
        app_engine: &Rc<RefCell<AppEngine>>,
        toast_overlay: &adw::ToastOverlay,
        service_uuid: String,
    ) {
        let container = container.clone();
        let app_engine = app_engine.clone();
        let toast_overlay = toast_overlay.clone();
        let (tx, rx) = mpsc::channel::<Result<Vec<ExchangeHardwareEvent>, String>>();

        let scan_toast = adw::Toast::new("Scanning for nearby devices…");
        scan_toast.set_timeout(3);
        toast_overlay.add_toast(scan_toast);

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tx.send(Err(format!("Tokio runtime: {}", e))).ok();
                    return;
                }
            };
            let result = rt.block_on(scan_for_devices(&service_uuid));
            tx.send(result).ok();
        });

        glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
            match rx.try_recv() {
                Ok(Ok(events)) => {
                    for event in events {
                        if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                            handle_app_engine_result(
                                &container,
                                &app_engine,
                                &toast_overlay,
                                result,
                            );
                        }
                    }
                    glib::ControlFlow::Break
                }
                Ok(Err(e)) => {
                    let event = ExchangeHardwareEvent::HardwareError {
                        transport: "BLE".into(),
                        error: e.clone(),
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                    }
                    let toast = adw::Toast::new(&format!("BLE scan failed: {}", e));
                    toast_overlay.add_toast(toast);
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
            }
        });
    }

    /// Start advertising our vauchi BLE service.
    pub fn start_advertising(toast_overlay: &adw::ToastOverlay, service_uuid: String) {
        let toast_overlay = toast_overlay.clone();
        let (tx, rx) = mpsc::channel::<Result<(), String>>();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tx.send(Err(format!("Tokio runtime: {}", e))).ok();
                    return;
                }
            };
            let result = rt.block_on(advertise_service(&service_uuid));
            tx.send(result).ok();
        });

        glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    let toast = adw::Toast::new("BLE advertising started");
                    toast_overlay.add_toast(toast);
                    glib::ControlFlow::Break
                }
                Ok(Err(e)) => {
                    let toast = adw::Toast::new(&format!("BLE advertise failed: {}", e));
                    toast_overlay.add_toast(toast);
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
            }
        });
    }

    /// Connect to a BLE device and report the result.
    pub fn connect(
        container: &gtk4::Box,
        app_engine: &Rc<RefCell<AppEngine>>,
        toast_overlay: &adw::ToastOverlay,
        device_id: String,
    ) {
        let container = container.clone();
        let app_engine = app_engine.clone();
        let toast_overlay = toast_overlay.clone();
        let (tx, rx) = mpsc::channel::<Result<(), String>>();

        let device_id_for_thread = device_id.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tx.send(Err(format!("Tokio runtime: {}", e))).ok();
                    return;
                }
            };
            let result = rt.block_on(connect_device(&device_id_for_thread));
            tx.send(result).ok();
        });

        let device_id_for_event = device_id;
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    let event = ExchangeHardwareEvent::BleConnected {
                        device_id: device_id_for_event.clone(),
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                    }
                    glib::ControlFlow::Break
                }
                Ok(Err(e)) => {
                    let event = ExchangeHardwareEvent::HardwareError {
                        transport: "BLE".into(),
                        error: e,
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                    }
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
            }
        });
    }

    /// Write data to a GATT characteristic.
    pub fn write_characteristic(
        container: &gtk4::Box,
        app_engine: &Rc<RefCell<AppEngine>>,
        toast_overlay: &adw::ToastOverlay,
        uuid: String,
        data: Vec<u8>,
    ) {
        let container = container.clone();
        let app_engine = app_engine.clone();
        let toast_overlay = toast_overlay.clone();
        let (tx, rx) = mpsc::channel::<Result<(), String>>();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tx.send(Err(format!("Tokio runtime: {}", e))).ok();
                    return;
                }
            };
            let result = rt.block_on(write_char(&uuid, &data));
            tx.send(result).ok();
        });

        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            match rx.try_recv() {
                Ok(Ok(())) => glib::ControlFlow::Break,
                Ok(Err(e)) => {
                    let event = ExchangeHardwareEvent::HardwareError {
                        transport: "BLE".into(),
                        error: e,
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                    }
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
            }
        });
    }

    /// Read data from a GATT characteristic.
    pub fn read_characteristic(
        container: &gtk4::Box,
        app_engine: &Rc<RefCell<AppEngine>>,
        toast_overlay: &adw::ToastOverlay,
        uuid: String,
    ) {
        let container = container.clone();
        let app_engine = app_engine.clone();
        let toast_overlay = toast_overlay.clone();
        let uuid_for_event = uuid.clone();
        let (tx, rx) = mpsc::channel::<Result<Vec<u8>, String>>();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tx.send(Err(format!("Tokio runtime: {}", e))).ok();
                    return;
                }
            };
            let result = rt.block_on(read_char(&uuid));
            tx.send(result).ok();
        });

        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            match rx.try_recv() {
                Ok(Ok(data)) => {
                    let event = ExchangeHardwareEvent::BleCharacteristicRead {
                        uuid: uuid_for_event.clone(),
                        data,
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                    }
                    glib::ControlFlow::Break
                }
                Ok(Err(e)) => {
                    let event = ExchangeHardwareEvent::HardwareError {
                        transport: "BLE".into(),
                        error: e,
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                    }
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
            }
        });
    }

    /// Disconnect from the current BLE device.
    pub fn disconnect(toast_overlay: &adw::ToastOverlay) {
        // BlueZ handles disconnect when adapter/device is dropped.
        // For explicit disconnect, we'd cache the device handle in a
        // session struct. For now, just notify the user.
        let toast = adw::Toast::new("BLE disconnected");
        toast_overlay.add_toast(toast);
    }

    // ── Async BlueZ operations ──────────────────────────────────────

    async fn scan_for_devices(service_uuid: &str) -> Result<Vec<ExchangeHardwareEvent>, String> {
        use bluer::AdapterEvent;
        use tokio_stream::StreamExt;

        let session = bluer::Session::new()
            .await
            .map_err(|e| format!("BlueZ session: {}", e))?;
        let adapter = session
            .default_adapter()
            .await
            .map_err(|e| format!("No BLE adapter: {}", e))?;
        adapter
            .set_powered(true)
            .await
            .map_err(|e| format!("Power on adapter: {}", e))?;

        let discover = adapter
            .discover_devices()
            .await
            .map_err(|e| format!("Start discovery: {}", e))?;

        let target_uuid: bluer::Uuid = service_uuid
            .parse()
            .map_err(|e| format!("Invalid UUID: {}", e))?;

        let mut events = Vec::new();
        let mut stream = discover;

        // Scan for up to 5 seconds
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);

        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                item = stream.next() => {
                    match item {
                        Some(AdapterEvent::DeviceAdded(addr)) => {
                            if let Ok(device) = adapter.device(addr) {
                                let uuids = device.uuids().await.unwrap_or(None).unwrap_or_default();
                                if uuids.contains(&target_uuid) {
                                    let rssi = device.rssi().await.unwrap_or(None).unwrap_or(0);
                                    events.push(ExchangeHardwareEvent::BleDeviceDiscovered {
                                        id: addr.to_string(),
                                        rssi,
                                        adv_data: vec![],
                                    });
                                }
                            }
                        }
                        None => break,
                        _ => {}
                    }
                }
            }
        }

        if events.is_empty() {
            Err("No vauchi devices found nearby".into())
        } else {
            Ok(events)
        }
    }

    async fn advertise_service(service_uuid: &str) -> Result<(), String> {
        use bluer::adv::Advertisement;

        let session = bluer::Session::new()
            .await
            .map_err(|e| format!("BlueZ session: {}", e))?;
        let adapter = session
            .default_adapter()
            .await
            .map_err(|e| format!("No BLE adapter: {}", e))?;
        adapter
            .set_powered(true)
            .await
            .map_err(|e| format!("Power on adapter: {}", e))?;

        let target_uuid: bluer::Uuid = service_uuid
            .parse()
            .map_err(|e| format!("Invalid UUID: {}", e))?;

        let adv = Advertisement {
            advertisement_type: bluer::adv::Type::Peripheral,
            service_uuids: vec![target_uuid].into_iter().collect(),
            local_name: Some("Vauchi".into()),
            ..Default::default()
        };

        let _handle = adapter
            .advertise(adv)
            .await
            .map_err(|e| format!("Start advertising: {}", e))?;

        // Keep advertising for 30 seconds
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

        Ok(())
    }

    async fn connect_device(device_id: &str) -> Result<(), String> {
        let addr: bluer::Address = device_id
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?;

        let session = bluer::Session::new()
            .await
            .map_err(|e| format!("BlueZ session: {}", e))?;
        let adapter = session
            .default_adapter()
            .await
            .map_err(|e| format!("No BLE adapter: {}", e))?;

        let device = adapter.device(addr).map_err(|e| format!("Device: {}", e))?;
        device
            .connect()
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        Ok(())
    }

    async fn write_char(uuid: &str, data: &[u8]) -> Result<(), String> {
        // In a full implementation, we'd cache the connected device and
        // look up the characteristic by UUID on its GATT services.
        // For now, this is a placeholder that will be wired when the
        // BLE session management is complete.
        let _ = (uuid, data);
        Err("GATT write requires active connection (session management TODO)".into())
    }

    async fn read_char(uuid: &str) -> Result<Vec<u8>, String> {
        let _ = uuid;
        Err("GATT read requires active connection (session management TODO)".into())
    }
}

#[cfg(feature = "ble")]
pub use inner::*;
