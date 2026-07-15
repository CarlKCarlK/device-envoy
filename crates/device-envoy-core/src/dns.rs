//! A device abstraction for resolving hostnames.

use core::future::Future;

use embassy_net::{Stack, dns::DnsQueryType};

/// One resolved address (embassy-net's address type re-exported for callers).
pub use embassy_net::IpAddress;

/// Addresses returned by a single lookup.
pub type Addresses = heapless::Vec<IpAddress, 4>;

/// Platform-independent contract for resolving a hostname.
pub trait Dns {
    /// Error returned by the platform DNS implementation.
    type Error;

    /// Resolve `hostname` to zero or more addresses.
    fn resolve(&mut self, hostname: &str) -> impl Future<Output = Result<Addresses, Self::Error>>;
}

/// A DNS resolver backed by an embassy-net stack.
pub struct DnsWithStack<'a> {
    stack: Stack<'a>,
}

impl<'a> DnsWithStack<'a> {
    /// Create a resolver backed by `stack`.
    pub const fn new(stack: Stack<'a>) -> Self {
        Self { stack }
    }
}

impl Dns for DnsWithStack<'_> {
    type Error = embassy_net::dns::Error;

    async fn resolve(&mut self, hostname: &str) -> Result<Addresses, Self::Error> {
        let resolved_addresses = self.stack.dns_query(hostname, DnsQueryType::A).await?;
        let mut addresses = Addresses::new();
        for address in resolved_addresses {
            if addresses.push(address).is_err() {
                break;
            }
        }
        Ok(addresses)
    }
}
