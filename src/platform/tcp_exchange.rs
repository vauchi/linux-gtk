// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! TCP client for USB cable exchange (ADR-031).
//!
//! Connects to the phone's TCP listener, executes the VXCH
//! framing protocol, and returns the peer's payload.

use std::net::TcpStream;
use std::time::Duration;

use vauchi_core::exchange::tcp_transport::TcpDirectTransport;

/// Default TCP port for vauchi USB exchange.
pub const USB_EXCHANGE_PORT: u16 = 19283;

/// Connect and read/write timeout.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Execute a direct exchange over TCP.
///
/// Connects to `addr`, exchanges payloads using VXCH framing,
/// and returns the peer's payload.
pub fn execute_exchange(
    addr: &str,
    our_payload: &[u8],
    is_initiator: bool,
) -> Result<Vec<u8>, String> {
    let sock_addr = addr.parse().map_err(|e| format!("bad address: {e}"))?;
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
