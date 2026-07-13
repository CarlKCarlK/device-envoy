use std::path::Path;

#[cfg(not(test))]
use std::{env, fs, path::PathBuf};

use resvg::{tiny_skia, usvg};

#[cfg(not(test))]
fn main() {
    let manifest_directory =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let asset_directory = manifest_directory.join("docs/assets/dns_tester");
    let output_directory = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));

    for asset in ["dns_landscape.svg", "dns_portrait.svg", "dynamic_layout.md"] {
        println!(
            "cargo:rerun-if-changed={}",
            asset_directory.join(asset).display()
        );
    }

    for (svg_name, tga_name, width, height) in [
        ("dns_landscape.svg", "dns_landscape.tga", 320, 240),
        ("dns_portrait.svg", "dns_portrait.tga", 240, 320),
    ] {
        let source = asset_directory.join(svg_name);
        let svg = fs::read_to_string(&source)
            .unwrap_or_else(|error| panic!("read {}: {error}", source.display()));
        let bytes = rasterize_svg(&svg, &source, width, height);
        fs::write(output_directory.join(tga_name), bytes)
            .unwrap_or_else(|error| panic!("write {tga_name}: {error}"));
    }
}

pub fn rasterize_svg(svg: &str, source: &Path, width: u32, height: u32) -> Vec<u8> {
    let (svg, removed_preview_group_count) = remove_preview_groups(svg, source);
    assert_eq!(
        removed_preview_group_count,
        1,
        "{} must contain exactly one preview-values group",
        source.display()
    );
    let tree = usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default())
        .unwrap_or_else(|error| panic!("parse {}: {error}", source.display()));
    assert_eq!(
        tree.size().width(),
        width as f32,
        "{} has the wrong SVG width; expected {width}",
        source.display()
    );
    assert_eq!(
        tree.size().height(),
        height as f32,
        "{} has the wrong SVG height; expected {height}",
        source.display()
    );
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .unwrap_or_else(|| panic!("allocate {width}x{height} raster for {}", source.display()));
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    encode_tga(width, height, pixmap.data())
}

pub fn remove_preview_groups(svg: &str, source: &Path) -> (String, usize) {
    let mut result = String::with_capacity(svg.len());
    let mut cursor = 0;
    let mut removed_group_count = 0;
    while let Some((relative_start, is_preview_group)) = [
        ("<g id=\"preview-values", true),
        ("<g id=\"developer-guides", false),
    ]
    .into_iter()
    .filter_map(|(prefix, is_preview_group)| {
        svg[cursor..]
            .find(prefix)
            .map(|relative_start| (relative_start, is_preview_group))
    })
    .min_by_key(|(relative_start, _)| *relative_start)
    {
        let start = cursor + relative_start;
        result.push_str(&svg[cursor..start]);
        let mut depth = 0;
        let mut scan = start;
        loop {
            let Some(open) = svg[scan..].find('<') else {
                panic!("unterminated preview group in {}", source.display());
            };
            let open = scan + open;
            let close = svg[open..]
                .find('>')
                .map(|offset| open + offset)
                .unwrap_or_else(|| panic!("unterminated SVG tag in {}", source.display()));
            let tag = &svg[open..=close];
            if tag.starts_with("<g ") || tag == "<g>" {
                depth += 1;
            } else if tag.starts_with("</g") {
                depth -= 1;
                if depth == 0 {
                    cursor = close + 1;
                    break;
                }
            }
            scan = close + 1;
        }
        if is_preview_group {
            removed_group_count += 1;
        }
    }
    result.push_str(&svg[cursor..]);
    (result, removed_group_count)
}

pub fn encode_tga(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    assert_eq!(rgba.len(), (width * height * 4) as usize);
    let mut tga = Vec::with_capacity(18 + (width * height * 3) as usize);
    tga.extend_from_slice(&[0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    tga.extend_from_slice(&(width as u16).to_le_bytes());
    tga.extend_from_slice(&(height as u16).to_le_bytes());
    tga.extend_from_slice(&[24, 0x20]);
    for pixel in rgba.chunks_exact(4) {
        tga.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
    }
    tga
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_one_nested_preview_group() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1">
            <rect width="2" height="1" fill="red"/>
            <g id="preview-values"><g><rect width="1" height="1" fill="blue"/></g></g>
            <g id="developer-guides"><rect width="1" height="1" fill="green"/></g>
        </svg>"#;
        let (filtered, count) = remove_preview_groups(svg, Path::new("fixture.svg"));
        assert_eq!(count, 1);
        assert!(!filtered.contains("preview-values"));
        assert!(!filtered.contains("developer-guides"));
        assert!(filtered.contains("fill=\"red\""));
    }

    #[test]
    fn tga_encoding_is_deterministic_and_top_origin_bgr() {
        let rgba = [1, 2, 3, 255, 4, 5, 6, 255];
        let first = encode_tga(2, 1, &rgba);
        let second = encode_tga(2, 1, &rgba);
        assert_eq!(first, second);
        assert_eq!(
            &first[..18],
            &[0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 1, 0, 24, 0x20]
        );
        assert_eq!(&first[18..], &[3, 2, 1, 6, 5, 4]);
    }

    #[test]
    fn missing_preview_metadata_panics_with_source_path() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"/>"#;
        let result = std::panic::catch_unwind(|| {
            rasterize_svg(svg, Path::new("missing-metadata.svg"), 2, 1);
        });
        let panic = match result {
            Ok(_) => panic!("missing metadata must fail the build"),
            Err(panic) => panic,
        };
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap_or_else(|| "panic did not contain a string message");
        assert!(message.contains("missing-metadata.svg"));
        assert!(message.contains("exactly one preview-values group"));
    }

    #[test]
    fn wrong_dimensions_panics_with_expected_size() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1">
            <g id="preview-values"><rect width="1" height="1"/></g>
        </svg>"#;
        let result = std::panic::catch_unwind(|| {
            rasterize_svg(svg, Path::new("wrong-size.svg"), 3, 1);
        });
        let panic = match result {
            Ok(_) => panic!("wrong dimensions must fail the build"),
            Err(panic) => panic,
        };
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap_or_else(|| "panic did not contain a string message");
        assert!(message.contains("wrong-size.svg has the wrong SVG width; expected 3"));
    }
}
