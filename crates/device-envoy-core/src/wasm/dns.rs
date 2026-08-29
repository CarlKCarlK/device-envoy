//! Deterministic DNS support for browser demos and tests.
//!
//! ```rust,no_run
//! use core::convert::Infallible;
//! use device_envoy_core::{
//!     dns::{Dns, IpAddress},
//!     wasm::DnsSimulatorWasm,
//! };
//! use embassy_time::Duration;
//!
//! async fn resolve() -> Result<(), Infallible> {
//!     let mut dns = DnsSimulatorWasm::new(
//!         [IpAddress::Ipv4([192, 0, 2, 1].into())],
//!         Duration::from_millis(1),
//!     );
//!     let addresses = dns.resolve("example.com").await?;
//!     assert_eq!(addresses.as_slice(), &[IpAddress::Ipv4([192, 0, 2, 1].into())]);
//!     let mut standard = DnsSimulatorWasm::standard();
//!     assert!(!standard.resolve("localhost").await?.is_empty());
//!     Ok(())
//! }
//! ```

use core::convert::Infallible;

use embassy_time::{Duration, Timer};

use crate::dns::{Addresses, Dns, IpAddress};

const SIMULATED_DNS_LATENCY: Duration = Duration::from_millis(12);

/// A fixed-address DNS implementation for environments without browser DNS.
/// See the compiled [`crate::wasm::dns`] example.
pub struct DnsSimulatorWasm {
    addresses: Addresses,
    latency: Duration,
}

impl DnsSimulatorWasm {
    /// Construct a resolver that returns `addresses` for every hostname.
    /// See the compiled [`crate::wasm::dns`] example.
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
    /// See the compiled [`crate::wasm::dns`] example.
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
