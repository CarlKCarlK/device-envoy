#![cfg(feature = "host")]

const LANDSCAPE_TGA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/dns_landscape.tga"));
const PORTRAIT_TGA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/dns_portrait.tga"));

#[test]
fn generated_backgrounds_are_uncompressed_top_origin_true_color_tga() {
    assert_tga_header(LANDSCAPE_TGA, 320, 240);
    assert_tga_header(PORTRAIT_TGA, 240, 320);
}

fn assert_tga_header(bytes: &[u8], width: u16, height: u16) {
    assert_eq!(
        bytes.len(),
        18 + usize::from(width) * usize::from(height) * 3
    );
    assert_eq!(&bytes[0..3], &[0, 0, 2]);
    assert_eq!(&bytes[12..14], &width.to_le_bytes());
    assert_eq!(&bytes[14..16], &height.to_le_bytes());
    assert_eq!(bytes[16], 24);
    assert_eq!(bytes[17], 0x20);
}
