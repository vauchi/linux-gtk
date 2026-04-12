// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! TCP client for USB cable exchange (ADR-031).
//!
//! Connects to the phone's TCP listener, executes the VXCH
//! framing protocol, and returns the peer's payload.
//!
//! Before connecting, `execute_exchange` attempts mDNS discovery via
//! `discover_phone()` to locate the phone's IP on the local network.
//! If discovery finds the service, the resolved address takes priority
//! over the caller-supplied `addr`. If discovery is disabled or times out,
//! the caller-supplied address is used as-is.

use std::net::TcpStream;
use std::time::Duration;

use vauchi_core::exchange::tcp_transport::TcpDirectTransport;

#[cfg(feature = "mdns")]
use mdns_sd::{ServiceDaemon, ServiceEvent};

/// Default TCP port for vauchi USB exchange.
pub const USB_EXCHANGE_PORT: u16 = 19283;

/// Connect and read/write timeout.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Discover the phone's address via mDNS/DNS-SD.
///
/// Browses for `_vauchi-exchange._tcp.local.` for up to 5 seconds.
/// Returns the first resolved address (IP:port), or None if not found.
#[cfg(feature = "mdns")]
pub fn discover_phone() -> Option<String> {
    let mdns = ServiceDaemon::new().ok()?;
    let receiver = mdns.browse("_vauchi-exchange._tcp.local.").ok()?;

    let deadline = std::time::Instant::now() + Duration::from_secs(5);

    while std::time::Instant::now() < deadline {
        match receiver.recv_timeout(Duration::from_millis(500)) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                if let Some(addr) = info.get_addresses().iter().next() {
                    let port = info.get_port();
                    mdns.shutdown().ok();
                    return Some(format!("{addr}:{port}"));
                }
            }
            Ok(_) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        }
    }

    mdns.shutdown().ok();
    None
}

#[cfg(not(feature = "mdns"))]
pub fn discover_phone() -> Option<String> {
    None
}

/// Execute a direct exchange over TCP.
///
/// Attempts mDNS discovery first to resolve the phone's address on the
/// local network. Falls back to the caller-supplied `addr` if discovery
/// is disabled or times out.
pub fn execute_exchange(
    addr: &str,
    our_payload: &[u8],
    is_initiator: bool,
) -> Result<Vec<u8>, String> {
    // Try mDNS discovery first, fall back to provided address.
    let resolved = discover_phone().unwrap_or_else(|| addr.to_string());

    let sock_addr = resolved.parse().map_err(|e| format!("bad address: {e}"))?;
    let stream = TcpStream::connect_timeout(&sock_addr, CONNECT_TIMEOUT)
        .map_err(|e| format!("TCP connect failed: {e}"))?;

    stream
        .set_read_timeout(Some(CONNECT_TIMEOUT))
        .map_err(|e| format!("set timeout: {e}"))?;
    stream
        .set_write_timeout(Some(CONNECT_TIMEOUT))
        .map_err(|e| format!("set timeout: {e}"))?;

    let mut transport = TcpDirectTransport::physical(stream);
    transport
        .exchange(our_payload, is_initiator)
        .map_err(|e| format!("exchange failed: {e}"))
}
