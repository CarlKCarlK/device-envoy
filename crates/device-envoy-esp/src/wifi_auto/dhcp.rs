//! Simple DHCP server for captive portal mode.
//!
//! Provides short leases in the portal subnet so phones can reach setup UI.

#![allow(clippy::future_not_send, reason = "single-threaded")]

use embassy_net::{
    Ipv4Address, Stack,
    udp::{self, UdpSocket},
};
use embassy_time::{Duration, Instant};
use log::{error, info, warn};

const DHCP_SERVER_PORT: u16 = 67;
const DHCP_MAGIC_COOKIE: [u8; 4] = [99, 130, 83, 99];
const DHCP_LEASE_SECONDS: u32 = 30;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DhcpMessageType {
    Discover,
    Request,
    Decline,
    Release,
    Inform,
    Other(u8),
}

struct DhcpMessage {
    msg_type: DhcpMessageType,
    transaction_id: u32,
    hardware_type: u8,
    hardware_len: u8,
    flags: u16,
    client_mac: [u8; 6],
    requested_ip: Option<Ipv4Address>,
}

struct DhcpLease {
    mac: [u8; 6],
    ip: Ipv4Address,
    expires_at: Instant,
}

fn parse_dhcp_message(frame: &[u8]) -> Option<DhcpMessage> {
    if frame.len() < 240 || frame[0] != 1 {
        return None;
    }

    let hardware_type = frame[1];
    let hardware_len = frame[2];
    if hardware_type != 1 || hardware_len != 6 {
        return None;
    }

    let transaction_id = u32::from_be_bytes([frame[4], frame[5], frame[6], frame[7]]);
    let flags = u16::from_be_bytes([frame[10], frame[11]]);

    if frame[236..240] != DHCP_MAGIC_COOKIE {
        return None;
    }

    let mut msg_type = None;
    let mut requested_ip = None;
    let mut option_index = 240;
    while option_index < frame.len() {
        let code = frame[option_index];
        option_index += 1;
        match code {
            0 => continue,
            255 => break,
            _ => {
                if option_index >= frame.len() {
                    break;
                }
                let option_len = frame[option_index] as usize;
                option_index += 1;
                if option_index + option_len > frame.len() {
                    break;
                }
                let option_data = &frame[option_index..option_index + option_len];
                match code {
                    50 if option_len == 4 => {
                        requested_ip = Some(Ipv4Address::new(
                            option_data[0],
                            option_data[1],
                            option_data[2],
                            option_data[3],
                        ));
                    }
                    53 if option_len == 1 => {
                        msg_type = Some(match option_data[0] {
                            1 => DhcpMessageType::Discover,
                            3 => DhcpMessageType::Request,
                            4 => DhcpMessageType::Decline,
                            7 => DhcpMessageType::Release,
                            8 => DhcpMessageType::Inform,
                            other => DhcpMessageType::Other(other),
                        });
                    }
                    _ => {}
                }
                option_index += option_len;
            }
        }
    }

    let mut client_mac = [0u8; 6];
    client_mac.copy_from_slice(&frame[28..34]);

    Some(DhcpMessage {
        msg_type: msg_type?,
        transaction_id,
        hardware_type,
        hardware_len,
        flags,
        client_mac,
        requested_ip,
    })
}

fn append_option(bytes: &mut [u8], code: u8, payload: &[u8]) -> Option<usize> {
    let needed = payload.len() + 2;
    if bytes.len() < needed {
        return None;
    }
    bytes[0] = code;
    bytes[1] = payload.len() as u8;
    bytes[2..2 + payload.len()].copy_from_slice(payload);
    Some(needed)
}

fn build_dhcp_reply(
    bytes: &mut [u8],
    request: &DhcpMessage,
    offered_ip: Ipv4Address,
    server_ip: Ipv4Address,
    netmask: Ipv4Address,
    broadcast_ip: Ipv4Address,
    response_kind: DhcpMessageType,
) -> Option<usize> {
    if bytes.len() < 300 {
        return None;
    }

    bytes.fill(0);
    bytes[0] = 2;
    bytes[1] = request.hardware_type;
    bytes[2] = request.hardware_len;
    bytes[4..8].copy_from_slice(&request.transaction_id.to_be_bytes());
    bytes[10..12].copy_from_slice(&request.flags.to_be_bytes());
    bytes[16..20].copy_from_slice(&offered_ip.octets());
    let server_ip_bytes = server_ip.octets();
    bytes[20..24].copy_from_slice(&server_ip_bytes);
    bytes[28..34].copy_from_slice(&request.client_mac);
    bytes[236..240].copy_from_slice(&DHCP_MAGIC_COOKIE);

    let lease_seconds = DHCP_LEASE_SECONDS;
    let renew_seconds = lease_seconds / 2;
    let rebind_seconds = (lease_seconds as u64 * 7 / 8) as u32;

    let mut option_index = 240;
    option_index += append_option(
        &mut bytes[option_index..],
        53,
        &[match response_kind {
            DhcpMessageType::Discover => 2,
            DhcpMessageType::Request => 5,
            DhcpMessageType::Other(code) => code,
            DhcpMessageType::Decline => 6,
            DhcpMessageType::Release => 7,
            DhcpMessageType::Inform => 8,
        }],
    )?;
    option_index += append_option(&mut bytes[option_index..], 54, &server_ip_bytes)?;
    option_index += append_option(&mut bytes[option_index..], 51, &lease_seconds.to_be_bytes())?;
    option_index += append_option(&mut bytes[option_index..], 58, &renew_seconds.to_be_bytes())?;
    option_index += append_option(
        &mut bytes[option_index..],
        59,
        &rebind_seconds.to_be_bytes(),
    )?;
    option_index += append_option(&mut bytes[option_index..], 1, &netmask.octets())?;
    option_index += append_option(&mut bytes[option_index..], 3, &server_ip_bytes)?;
    option_index += append_option(&mut bytes[option_index..], 6, &server_ip_bytes)?;
    option_index += append_option(&mut bytes[option_index..], 28, &broadcast_ip.octets())?;
    bytes[option_index] = 255;
    option_index += 1;
    Some(option_index)
}

fn ip_in_pool(ip: Ipv4Address, pool_start: Ipv4Address, pool_size: u8) -> bool {
    if pool_size == 0 {
        return false;
    }
    let start_u32 = u32::from_be_bytes(pool_start.octets());
    let end_u32 = start_u32 + pool_size as u32 - 1;
    let candidate_u32 = u32::from_be_bytes(ip.octets());
    candidate_u32 >= start_u32 && candidate_u32 <= end_u32
}

fn ensure_lease(
    leases: &mut heapless::Vec<DhcpLease, 8>,
    mac: [u8; 6],
    pool_start: Ipv4Address,
    pool_size: u8,
    requested_ip: Option<Ipv4Address>,
) -> Option<Ipv4Address> {
    let now = Instant::now();
    leases.retain(|lease| lease.expires_at > now);
    let expiry = now + Duration::from_secs(DHCP_LEASE_SECONDS as u64);

    let desired_ip = requested_ip
        .filter(|ip| ip_in_pool(*ip, pool_start, pool_size))
        .filter(|ip| {
            leases
                .iter()
                .all(|lease| lease.mac == mac || lease.ip != *ip)
        });

    if let Some(existing_lease) = leases.iter_mut().find(|lease| lease.mac == mac) {
        if let Some(ip) = desired_ip {
            existing_lease.ip = ip;
        }
        existing_lease.expires_at = expiry;
        return Some(existing_lease.ip);
    }

    let free_ip = desired_ip.or_else(|| {
        let pool_start_u32 = u32::from_be_bytes(pool_start.octets());
        for offset in 0..pool_size {
            let candidate_u32 = pool_start_u32 + offset as u32;
            let octets = candidate_u32.to_be_bytes();
            let candidate_ip = Ipv4Address::new(octets[0], octets[1], octets[2], octets[3]);
            if leases.iter().all(|lease| lease.ip != candidate_ip) {
                return Some(candidate_ip);
            }
        }
        None
    })?;

    leases
        .push(DhcpLease {
            mac,
            ip: free_ip,
            expires_at: expiry,
        })
        .ok()?;
    Some(free_ip)
}

pub async fn dhcp_server_task(
    stack: Stack<'static>,
    server_ip: Ipv4Address,
    netmask: Ipv4Address,
    pool_start: Ipv4Address,
    pool_size: u8,
) -> ! {
    let mut rx_metadata = [udp::PacketMetadata::EMPTY; 4];
    let mut rx_buffer = [0u8; 768];
    let mut tx_metadata = [udp::PacketMetadata::EMPTY; 4];
    let mut tx_buffer = [0u8; 768];
    let mut socket = UdpSocket::new(
        stack,
        &mut rx_metadata,
        &mut rx_buffer,
        &mut tx_metadata,
        &mut tx_buffer,
    );

    if let Err(error) = socket.bind(DHCP_SERVER_PORT) {
        error!("DHCP bind failed: {:?}", error);
        panic!("DHCP server bind failed");
    }

    let broadcast_ip = Ipv4Address::new(
        server_ip.octets()[0],
        server_ip.octets()[1],
        server_ip.octets()[2],
        255,
    );
    info!("DHCP server listening on {}", server_ip);

    let mut leases: heapless::Vec<DhcpLease, 8> = heapless::Vec::new();
    let mut frame = [0u8; 768];
    let mut response = [0u8; 768];

    loop {
        let (frame_len, _) = match socket.recv_from(&mut frame).await {
            Ok(data) => data,
            Err(error) => {
                warn!("DHCP recv error: {:?}", error);
                continue;
            }
        };

        let request = match parse_dhcp_message(&frame[..frame_len]) {
            Some(request) => request,
            None => continue,
        };

        let offered_ip = match ensure_lease(
            &mut leases,
            request.client_mac,
            pool_start,
            pool_size,
            request.requested_ip,
        ) {
            Some(ip) => ip,
            None => continue,
        };

        let response_kind = match request.msg_type {
            DhcpMessageType::Discover => DhcpMessageType::Discover,
            DhcpMessageType::Request => DhcpMessageType::Request,
            _ => continue,
        };

        let response_len = match build_dhcp_reply(
            &mut response,
            &request,
            offered_ip,
            server_ip,
            netmask,
            broadcast_ip,
            response_kind,
        ) {
            Some(size) => size,
            None => continue,
        };

        let target = embassy_net::IpEndpoint::new(embassy_net::IpAddress::Ipv4(broadcast_ip), 68);
        if let Err(error) = socket.send_to(&response[..response_len], target).await {
            warn!("DHCP send error: {:?}", error);
        }
    }
}
