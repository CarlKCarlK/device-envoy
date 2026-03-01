#[cfg(all(feature = "wifi", not(feature = "host")))]
pub(crate) use crate::wifi_auto::{Wifi, WifiEvent, WifiStatic};
