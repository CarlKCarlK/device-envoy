use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_directory =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let asset_directory = manifest_directory.join("docs/assets/dns_tester");
    let output_directory = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));

    for asset in [
        "dns_landscape.svg",
        "dns_portrait.svg",
        "dynamic_layout.md",
        "dns_landscape.tga",
        "dns_portrait.tga",
    ] {
        println!(
            "cargo:rerun-if-changed={}",
            asset_directory.join(asset).display()
        );
    }

    for (name, width, height) in [
        ("dns_landscape.tga", 320, 240),
        ("dns_portrait.tga", 240, 320),
    ] {
        let source = asset_directory.join(name);
        let bytes =
            fs::read(&source).unwrap_or_else(|error| panic!("read {}: {error}", source.display()));
        if bytes.len() < 18
            || bytes[2] != 2
            || bytes[16] != 24
            || u16::from_le_bytes([bytes[12], bytes[13]]) as usize != width
            || u16::from_le_bytes([bytes[14], bytes[15]]) as usize != height
        {
            panic!(
                "{} is not an uncompressed {width}x{height} true-color TGA",
                source.display()
            );
        }
        fs::write(output_directory.join(name), bytes)
            .unwrap_or_else(|error| panic!("write {name}: {error}"));
    }
}
