//! A device abstraction for choosing shared RMT initialization mode.
//!
//! Use [`Blocking`] or [`Async`] with `init_and_start!(p, rmt80, ...)`.

/// RMT mode selector for `init_and_start!(p, rmt80, ...)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RmtMode {
    /// Initialize shared RMT in blocking mode.
    Blocking,
    /// Initialize shared RMT in async mode.
    Async,
}

pub use RmtMode::{Async, Blocking};
