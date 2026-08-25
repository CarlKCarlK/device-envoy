fn feature_enabled(feature_name: &str) -> bool {
    let environment_name = format!("CARGO_FEATURE_{feature_name}");
    std::env::var_os(environment_name).is_some()
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg(esp_has_rmt)");

    // Keep this chip list aligned with device-envoy-esp/build.rs. The README
    // example uses this configuration while compiling in the examples crate.
    if feature_enabled("ESP32")
        || feature_enabled("ESP32C3")
        || feature_enabled("ESP32C6")
        || feature_enabled("ESP32H2")
        || feature_enabled("ESP32S2")
        || feature_enabled("ESP32S3")
    {
        println!("cargo::rustc-cfg=esp_has_rmt");
    }
}
