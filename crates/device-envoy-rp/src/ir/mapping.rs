//! A device abstraction for mapping IR remote buttons to application-specific actions.
//!
//! See [`IrMapping`](trait@crate::ir::IrMapping) and this module's macros for generated types.

use heapless::LinearMap;

// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub fn __build_button_map<B: Copy, const N: usize>(
    button_map: &[(u16, u8, B)],
) -> LinearMap<(u16, u8), B, N> {
    let mut linear_map = LinearMap::new();
    for &(addr, cmd, button) in button_map {
        let previous_button = match linear_map.insert((addr, cmd), button) {
            Ok(previous_button) => previous_button,
            Err(_) => panic!("button_map entries exceed IrMapping capacity"),
        };
        assert!(
            previous_button.is_none(),
            "button_map contains duplicate (addr, cmd) entries"
        );
    }
    linear_map
}
