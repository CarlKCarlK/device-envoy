//! Linker-script setup for the reusable RP platform crate.

use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(rust_analyzer)");

    let target = env::var("TARGET").expect("TARGET must be set by Cargo");
    let out_directory = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set by Cargo"));

    let memory_file = if target.starts_with("thumbv8m") {
        "memory-pico2.x"
    } else if target.starts_with("thumbv6m") {
        "memory-pico1w.x"
    } else if target.starts_with("riscv32") {
        panic!("RISC-V targets are not supported by device-envoy-rp");
    } else {
        return;
    };

    let memory_source = PathBuf::from(memory_file);
    let memory_contents = fs::read_to_string(&memory_source)
        .unwrap_or_else(|error| panic!("read {}: {error}", memory_source.display()));
    let memory_destination = out_directory.join("memory.x");
    fs::write(&memory_destination, memory_contents)
        .unwrap_or_else(|error| panic!("write {}: {error}", memory_destination.display()));
    println!("cargo:rustc-link-search={}", out_directory.display());
    println!("cargo:rerun-if-changed={memory_file}");
}
