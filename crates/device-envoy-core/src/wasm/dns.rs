//! Deterministic DNS support for browser demos and tests.

use core::convert::Infallible;

use embassy_time::{Duration, Timer};

use crate::dns::{Addresses, Dns, IpAddress};

const SIMULATED_DNS_LATENCY: Duration = Duration::from_millis(12);

/// A fixed-address DNS implementation for environments without browser DNS.
pub struct DnsFixedWasm {
    addresses: Addresses,
}

impl DnsFixedWasm {
    /// Construct a resolver that returns `addresses` for every hostname.
    pub fn new<const COUNT: usize>(addresses: [IpAddress; COUNT]) -> Self {
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
        }
    }
}

impl Dns for DnsFixedWasm {
    type Error = Infallible;

    async fn resolve(&mut self, _hostname: &str) -> Result<Addresses, Self::Error> {
        Timer::after(SIMULATED_DNS_LATENCY).await;
        Ok(self.addresses.clone())
    }
}
