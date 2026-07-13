//! A device abstraction for one measured DNS lookup.

use core::future::Future;

/// Result of one DNS lookup, including the adapter's measured duration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DnsLookupResult {
    /// Whether the lookup returned at least one address.
    pub succeeded: bool,
    /// Measured lookup duration in milliseconds.
    pub latency_millis: u64,
}

/// Platform-independent contract for the DNS operation used by examples.
pub trait DnsLookup {
    /// Error returned by the platform DNS implementation.
    type Error;

    /// Look up `hostname` and measure the operation duration.
    fn lookup(
        &mut self,
        hostname: &str,
    ) -> impl Future<Output = Result<DnsLookupResult, Self::Error>>;
}

/// Adapter for an async DNS lookup function.
pub struct DnsLookupFn<F>(pub F);

impl<F, Error> DnsLookup for DnsLookupFn<F>
where
    F: AsyncFnMut(&str) -> Result<DnsLookupResult, Error>,
{
    type Error = Error;

    fn lookup(
        &mut self,
        hostname: &str,
    ) -> impl Future<Output = Result<DnsLookupResult, Self::Error>> {
        (self.0)(hostname)
    }
}
