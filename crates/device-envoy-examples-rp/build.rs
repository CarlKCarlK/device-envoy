//! Cargo build script for the Pico examples package.

fn main() {
    println!("cargo:rustc-check-cfg=cfg(rust_analyzer)");
}
