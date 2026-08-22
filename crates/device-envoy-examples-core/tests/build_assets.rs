#[path = "../build.rs"]
mod build_script;

use std::path::Path;

#[test]
fn build_helpers_reject_missing_preview_metadata() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"/>"#;
    let result = std::panic::catch_unwind(|| {
        build_script::rasterize_svg(svg, Path::new("missing-metadata.svg"), 2, 1);
    });
    assert!(result.is_err());
}

#[test]
fn build_helpers_reject_wrong_dimensions() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1">
        <g id="preview-values"><rect width="1" height="1"/></g>
    </svg>"#;
    let result = std::panic::catch_unwind(|| {
        build_script::rasterize_svg(svg, Path::new("wrong-size.svg"), 3, 1);
    });
    assert!(result.is_err());
}

#[test]
fn build_helpers_emit_deterministic_tga_bytes() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1">
        <rect width="2" height="1" fill="red"/>
        <g id="preview-values"><rect width="1" height="1" fill="blue"/></g>
    </svg>"#;
    let first = build_script::rasterize_svg(svg, Path::new("deterministic.svg"), 2, 1);
    let second = build_script::rasterize_svg(svg, Path::new("deterministic.svg"), 2, 1);
    assert_eq!(first, second);
    assert_eq!(first[2], 2);
    assert_eq!(first[16], 24);
    assert_eq!(first[17], 0x20);
}
