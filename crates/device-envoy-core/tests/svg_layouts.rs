#![allow(missing_docs)]
#![cfg(feature = "host")]

use device_envoy_core::led2d::layout::LedLayout;
use std::error::Error;
use std::fmt::{self, Write as _};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const BACKGROUND_COLOR: &str = "#fbfcfe"; // Near-white cool gray.
const PANEL_COLOR: &str = "#f4f7fa"; // Pale blue-gray.
const PANEL_STROKE_COLOR: &str = "#d9e0e7"; // Light blue-gray.
const SEAM_COLOR: &str = "#b8c2cc"; // Muted blue-gray.
const PATH_COLOR: &str = "#db806c"; // Soft coral.
const LED_COLOR: &str = "#ffffff"; // White.
const LED_STROKE_COLOR: &str = "#9aa8b5"; // Medium blue-gray.
const LABEL_COLOR: &str = "#33424f"; // Dark slate.
const CELL_SIZE: usize = 72;
const PANEL_PADDING: usize = 26;

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);

#[test]
fn led_layout_12x8_serial_svg_matches_expected() -> Result<(), Box<dyn Error>> {
    const FILENAME: &str = "led_layout_12x8_serial.svg";
    let actual = render_led_layout_svg(&LED_LAYOUT_12X8, &[4])?;
    let expected_path = docs_assets_path(FILENAME);

    if std::env::var_os("DEVICE_KIT_UPDATE_SVGS").is_some() {
        fs::write(&expected_path, actual)?;
        println!("updated SVG at {}", expected_path.display());
        return Ok(());
    }
    if !expected_path.exists() {
        return Err(format!("expected SVG is missing at {}", expected_path.display()).into());
    }

    let output_path = temp_output_path(FILENAME);
    fs::write(&output_path, &actual)?;
    let expected = fs::read_to_string(&expected_path)?;
    let comparison = expected == actual;
    fs::remove_file(&output_path)?;
    assert!(comparison, "SVG text must match");
    Ok(())
}

fn render_led_layout_svg<const N: usize, const W: usize, const H: usize>(
    led_layout: &LedLayout<N, W, H>,
    horizontal_seams: &[usize],
) -> Result<String, fmt::Error> {
    assert!(led_layout.width() > 0, "layout width must be positive");
    assert!(led_layout.height() > 0, "layout height must be positive");
    assert!(led_layout.len() > 0, "layout must contain LEDs");
    for &seam_row in horizontal_seams {
        assert!(
            (1..led_layout.height()).contains(&seam_row),
            "seam row must lie inside the layout"
        );
    }

    const LED_RADIUS: usize = 21;
    const OUTER_PADDING: usize = 58;
    const CABLE_GUTTER: usize = 34;
    const CONTENT_PADDING: usize = 8;

    let panel_left = OUTER_PADDING + CABLE_GUTTER;
    let panel_top = OUTER_PADDING;
    let first_center_x = panel_left + CELL_SIZE / 2;
    let first_center_y = panel_top + CELL_SIZE / 2;
    let panel_width = led_layout.width() * CELL_SIZE;
    let panel_height = led_layout.height() * CELL_SIZE;
    let svg_width = panel_width + (OUTER_PADDING + CABLE_GUTTER) * 2;
    let svg_height = panel_height + OUTER_PADDING * 2;
    let viewbox_left = panel_left - CABLE_GUTTER - CONTENT_PADDING;
    let viewbox_top = panel_top + PANEL_PADDING / 2 - CONTENT_PADDING;
    let viewbox_width = panel_width + CABLE_GUTTER * 2 + CONTENT_PADDING * 2;
    let viewbox_height = panel_height - PANEL_PADDING + CONTENT_PADDING * 2;

    let mut svg = String::new();
    writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{viewbox_width}" height="{viewbox_height}" viewBox="{viewbox_left} {viewbox_top} {viewbox_width} {viewbox_height}" role="img" aria-labelledby="title description">"#
    )?;
    writeln!(
        svg,
        "  <title id=\"title\">{}×{} LED panel serial order</title>",
        led_layout.width(),
        led_layout.height()
    )?;
    writeln!(
        svg,
        "  <desc id=\"description\">{} LEDs labeled 0 through {}, connected in serial order. Horizontal rules show stacked panel seams.</desc>",
        led_layout.len(),
        led_layout.len() - 1
    )?;
    writeln!(
        svg,
        "  <rect width=\"{svg_width}\" height=\"{svg_height}\" fill=\"{BACKGROUND_COLOR}\"/>"
    )?;
    writeln!(svg, "  <defs>")?;
    writeln!(
        svg,
        "    <marker id=\"arrow\" viewBox=\"0 0 8 8\" refX=\"7\" refY=\"4\" markerWidth=\"5\" markerHeight=\"5\" orient=\"auto\">"
    )?;
    writeln!(
        svg,
        "      <path d=\"M 0 0 L 8 4 L 0 8 z\" fill=\"{PATH_COLOR}\"/>"
    )?;
    writeln!(svg, "    </marker>")?;
    writeln!(svg, "  </defs>")?;

    write_panels(
        &mut svg,
        led_layout.height(),
        horizontal_seams,
        panel_left,
        panel_top,
        panel_width,
    )?;

    writeln!(
        svg,
        "  <g fill=\"none\" stroke=\"{PATH_COLOR}\" stroke-width=\"3\" stroke-linecap=\"round\" stroke-linejoin=\"round\" opacity=\"0.58\">"
    )?;
    for serial_index in 0..led_layout.len() - 1 {
        let (from_x, from_y) = led_layout.index_to_xy()[serial_index];
        let (to_x, to_y) = led_layout.index_to_xy()[serial_index + 1];
        let from_x = first_center_x + usize::from(from_x) * CELL_SIZE;
        let from_y = first_center_y + usize::from(from_y) * CELL_SIZE;
        let to_x = first_center_x + usize::from(to_x) * CELL_SIZE;
        let to_y = first_center_y + usize::from(to_y) * CELL_SIZE;

        if let Some(seam_row) =
            crossed_seam(from_y, to_y, first_center_y, CELL_SIZE, horizontal_seams)
        {
            let seam_y = panel_top + seam_row * CELL_SIZE;
            let right_gutter_x = panel_left + panel_width + CABLE_GUTTER;
            let left_gutter_x = panel_left - CABLE_GUTTER;
            writeln!(
                svg,
                "    <path d=\"M {from_x} {from_y} H {right_gutter_x} V {seam_y} H {left_gutter_x} V {to_y} H {to_x}\"/>"
            )?;
            write_arrow_line(
                &mut svg,
                right_gutter_x - panel_width / 2 + 18,
                seam_y,
                right_gutter_x - panel_width / 2 - 18,
                seam_y,
            )?;
        } else {
            writeln!(
                svg,
                "    <line x1=\"{from_x}\" y1=\"{from_y}\" x2=\"{to_x}\" y2=\"{to_y}\"/>"
            )?;
            let arrow_start_x = weighted_position(from_x, to_x, 43);
            let arrow_start_y = weighted_position(from_y, to_y, 43);
            let arrow_end_x = weighted_position(from_x, to_x, 57);
            let arrow_end_y = weighted_position(from_y, to_y, 57);
            write_arrow_line(
                &mut svg,
                arrow_start_x,
                arrow_start_y,
                arrow_end_x,
                arrow_end_y,
            )?;
        }
    }
    writeln!(svg, "  </g>")?;

    writeln!(svg, "  <g text-anchor=\"middle\">")?;
    for (serial_index, &(position_x, position_y)) in led_layout.index_to_xy().iter().enumerate() {
        let center_x = first_center_x + usize::from(position_x) * CELL_SIZE;
        let center_y = first_center_y + usize::from(position_y) * CELL_SIZE;
        writeln!(
            svg,
            "    <circle cx=\"{center_x}\" cy=\"{center_y}\" r=\"{LED_RADIUS}\" fill=\"{LED_COLOR}\" stroke=\"{LED_STROKE_COLOR}\" stroke-width=\"2\"/>"
        )?;
        writeln!(
            svg,
            "    <text x=\"{center_x}\" y=\"{}\" fill=\"{LABEL_COLOR}\" font-family=\"ui-monospace, SFMono-Regular, Consolas, monospace\" font-size=\"13\" font-weight=\"500\">{serial_index}</text>",
            center_y + 5
        )?;
    }
    writeln!(svg, "  </g>")?;
    writeln!(svg, "</svg>")?;
    Ok(svg)
}

fn write_panels(
    svg: &mut String,
    height: usize,
    horizontal_seams: &[usize],
    panel_left: usize,
    panel_top: usize,
    panel_width: usize,
) -> Result<(), fmt::Error> {
    let mut first_row = 0;
    for last_row in horizontal_seams
        .iter()
        .copied()
        .chain(std::iter::once(height))
    {
        let top = panel_top + first_row * CELL_SIZE + PANEL_PADDING / 2;
        let panel_height = (last_row - first_row) * CELL_SIZE - PANEL_PADDING;
        writeln!(
            svg,
            "  <rect x=\"{panel_left}\" y=\"{top}\" width=\"{panel_width}\" height=\"{panel_height}\" rx=\"12\" fill=\"{PANEL_COLOR}\" stroke=\"{PANEL_STROKE_COLOR}\" stroke-width=\"2\"/>"
        )?;
        first_row = last_row;
    }

    for &seam_row in horizontal_seams {
        let seam_y = panel_top + seam_row * CELL_SIZE;
        writeln!(
            svg,
            "  <line x1=\"{panel_left}\" y1=\"{seam_y}\" x2=\"{}\" y2=\"{seam_y}\" stroke=\"{SEAM_COLOR}\" stroke-width=\"1.5\" stroke-dasharray=\"7 7\"/>",
            panel_left + panel_width
        )?;
    }
    Ok(())
}

fn write_arrow_line(
    svg: &mut String,
    from_x: usize,
    from_y: usize,
    to_x: usize,
    to_y: usize,
) -> Result<(), fmt::Error> {
    writeln!(
        svg,
        "    <line x1=\"{from_x}\" y1=\"{from_y}\" x2=\"{to_x}\" y2=\"{to_y}\" marker-end=\"url(#arrow)\"/>"
    )
}

fn crossed_seam(
    from_y: usize,
    to_y: usize,
    first_center_y: usize,
    cell_size: usize,
    horizontal_seams: &[usize],
) -> Option<usize> {
    horizontal_seams.iter().copied().find(|&seam_row| {
        let seam_y = first_center_y + seam_row * cell_size - cell_size / 2;
        (from_y < seam_y && to_y > seam_y) || (to_y < seam_y && from_y > seam_y)
    })
}

fn weighted_position(from: usize, to: usize, percentage: usize) -> usize {
    (from * (100 - percentage) + to * percentage) / 100
}

fn docs_assets_path(filename: &str) -> PathBuf {
    Path::new("docs").join("assets").join(filename)
}

fn temp_output_path(filename: &str) -> PathBuf {
    let unix_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be valid")
        .as_nanos();
    std::env::temp_dir().join(format!("{filename}-{}-{unix_time}", std::process::id()))
}
