//! A device abstraction for one measured DNS lookup.

use core::future::Future;

/// Result of one DNS lookup, including the adapter's measured duration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DnsResult {
    /// Whether the lookup returned at least one address.
    pub succeeded: bool,
    /// Measured lookup duration in milliseconds.
    pub latency_millis: u64,
}

/// Platform-independent contract for the DNS operation used by examples.
pub trait Dns {
    /// Error returned by the platform DNS implementation.
    type Error;

    /// Hostname used by the lookup operation.
    fn hostname(&self) -> &'static str;

    /// Look up the configured hostname and measure the operation duration.
    fn lookup(&mut self) -> impl Future<Output = Result<DnsResult, Self::Error>>;
}

/// Adapter for an async DNS lookup function.
pub struct DnsRuntime<F> {
    hostname: &'static str,
    lookup: F,
}

impl<F> DnsRuntime<F> {
    /// Create a lookup adapter for `hostname`.
    pub const fn new(hostname: &'static str, lookup: F) -> Self {
        Self { hostname, lookup }
    }
}

impl<F, Error> Dns for DnsRuntime<F>
where
    F: AsyncFnMut() -> Result<DnsResult, Error>,
{
    type Error = Error;

    fn hostname(&self) -> &'static str {
        self.hostname
    }

    fn lookup(&mut self) -> impl Future<Output = Result<DnsResult, Self::Error>> {
        (self.lookup)()
    }
}
