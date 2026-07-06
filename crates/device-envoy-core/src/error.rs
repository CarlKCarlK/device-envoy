//! Shared error and result types for `device-envoy-core`.

/// A specialized `Result` where the error is this crate's [`Error`] type.
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Unified error type for `device-envoy-core`.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Spawning an Embassy task failed.
    #[cfg(feature = "wifi")]
    TaskSpawn(embassy_executor::SpawnError),
}

#[cfg(feature = "wifi")]
impl From<embassy_executor::SpawnError> for Error {
    fn from(err: embassy_executor::SpawnError) -> Self {
        Self::TaskSpawn(err)
    }
}
