//! Simple DNS server for captive portal mode.
//!
//! Responds to all queries with the portal gateway IP.

#![allow(clippy::future_not_send, reason = "single-threaded")]

use embassy_net::{
    Ipv4Address, Stack,
    udp::{self, UdpSocket},
};
use log::{error, warn};

const DNS_SERVER_PORT: u16 = 53;

pub async fn dns_server_task(stack: Stack<'static>, answer_ip: Ipv4Address) -> ! {
    let mut rx_metadata = [udp::PacketMetadata::EMPTY; 4];
    let mut rx_buffer = [0u8; 512];
    let mut tx_metadata = [udp::PacketMetadata::EMPTY; 4];
    let mut tx_buffer = [0u8; 512];
    let mut socket = UdpSocket::new(
        stack,
        &mut rx_metadata,
        &mut rx_buffer,
        &mut tx_metadata,
        &mut tx_buffer,
    );

    if let Err(error) = socket.bind(DNS_SERVER_PORT) {
        error!("DNS bind failed: {:?}", error);
        panic!("DNS server bind failed");
    }

    let mut frame = [0u8; 512];
    loop {
        let (frame_len, remote) = match socket.recv_from(&mut frame).await {
            Ok(data) => data,
            Err(_) => continue,
        };

        if frame_len < 12 {
            continue;
        }

        let mut response = [0u8; 512];
        response[..frame_len].copy_from_slice(&frame[..frame_len]);
        response[2] = 0x84;
        response[3] = 0x00;
        response[6] = 0x00;
        response[7] = 0x01;

        let mut index = frame_len;
        if index + 16 > response.len() {
            continue;
        }
        response[index] = 0xC0;
        response[index + 1] = 0x0C;
        index += 2;
        response[index] = 0x00;
        response[index + 1] = 0x01;
        index += 2;
        response[index] = 0x00;
        response[index + 1] = 0x01;
        index += 2;
        response[index] = 0x00;
        response[index + 1] = 0x00;
        response[index + 2] = 0x00;
        response[index + 3] = 0x3C;
        index += 4;
        response[index] = 0x00;
        response[index + 1] = 0x04;
        index += 2;
        let answer_octets = answer_ip.octets();
        response[index..index + 4].copy_from_slice(&answer_octets);
        index += 4;

        if let Err(error) = socket.send_to(&response[..index], remote).await {
            warn!("DNS send error: {:?}", error);
        }
    }
}
