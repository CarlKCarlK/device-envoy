//! Deterministic DNS support for browser demos and tests.

use core::convert::Infallible;

use embassy_time::{Duration, Timer};

use crate::dns::{Addresses, Dns, IpAddress};

const SIMULATED_DNS_LATENCY: Duration = Duration::from_millis(12);

/// A fixed-address DNS implementation for environments without browser DNS.
pub struct DnsSimulatorWasm {
    addresses: Addresses,
    latency: Duration,
}

impl DnsSimulatorWasm {
    /// Construct a resolver that returns `addresses` for every hostname.
    pub fn new<const COUNT: usize>(addresses: [IpAddress; COUNT], latency: Duration) -> Self {
        assert!(
            COUNT <= 4,
            "DNS address list cannot contain more than four addresses"
        );
        let mut fixed_addresses = Addresses::new();
        for address in addresses {
            fixed_addresses
                .push(address)
                .expect("address count was checked against the DNS capacity");
        }
        Self {
            addresses: fixed_addresses,
            latency,
        }
    }

    /// Construct the standard browser resolver, which returns IPv4 loopback.
    pub fn standard() -> Self {
        Self::new(
            [IpAddress::Ipv4([127, 0, 0, 1].into())],
            SIMULATED_DNS_LATENCY,
        )
    }
}

impl Dns for DnsSimulatorWasm {
    type Error = Infallible;

    async fn resolve(&mut self, _hostname: &str) -> Result<Addresses, Self::Error> {
        Timer::after(self.latency).await;
        Ok(self.addresses.clone())
    }
}
