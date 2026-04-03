fn feature_enabled(feature_name: &str) -> bool {
    let env_name = format!("CARGO_FEATURE_{}", feature_name);
    std::env::var_os(env_name).is_some()
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(esp_pdma_family)");
    println!("cargo:rustc-check-cfg=cfg(esp_gdma_family)");
    println!("cargo:rustc-check-cfg=cfg(esp_has_rmt)");
    println!("cargo:rustc-check-cfg=cfg(esp_has_i2s)");
    println!("cargo:rustc-check-cfg=cfg(esp_has_wifi)");
    println!("cargo:rustc-check-cfg=cfg(rust_analyzer)");

    if feature_enabled("ESP32") || feature_enabled("ESP32S2") {
        println!("cargo:rustc-cfg=esp_pdma_family");
    }

    if feature_enabled("ESP32C2")
        || feature_enabled("ESP32C3")
        || feature_enabled("ESP32C6")
        || feature_enabled("ESP32H2")
        || feature_enabled("ESP32S3")
    {
        println!("cargo:rustc-cfg=esp_gdma_family");
    }

    if feature_enabled("ESP32")
        || feature_enabled("ESP32C3")
        || feature_enabled("ESP32C6")
        || feature_enabled("ESP32H2")
        || feature_enabled("ESP32S2")
        || feature_enabled("ESP32S3")
    {
        println!("cargo:rustc-cfg=esp_has_rmt");
        println!("cargo:rustc-cfg=esp_has_i2s");
    }

    if feature_enabled("ESP32")
        || feature_enabled("ESP32C2")
        || feature_enabled("ESP32C3")
        || feature_enabled("ESP32C6")
        || feature_enabled("ESP32S2")
        || feature_enabled("ESP32S3")
    {
        println!("cargo:rustc-cfg=esp_has_wifi");
    }
}
