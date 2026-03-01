#![allow(missing_docs)]
//! Host-level tests for mapping primitives.

use device_envoy::led2d::layout::LedLayout;

#[test]
fn linear_single_row_matches_expected() {
    const LINEAR: LedLayout<4, 4, 1> = LedLayout::new([(0, 0), (1, 0), (2, 0), (3, 0)]);
    assert_eq!(LINEAR.index_to_xy(), &[(0, 0), (1, 0), (2, 0), (3, 0)]);
}

#[test]
fn linear_single_column_matches_expected() {
    const LINEAR: LedLayout<4, 1, 4> = LedLayout::new([(0, 0), (0, 1), (0, 2), (0, 3)]);
    assert_eq!(LINEAR.index_to_xy(), &[(0, 0), (0, 1), (0, 2), (0, 3)]);
}

#[test]
fn linear_h_returns_expected() {
    const LINEAR: LedLayout<5, 5, 1> = LedLayout::linear_h();
    assert_eq!(
        LINEAR.index_to_xy(),
        &[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]
    );
}

#[test]
fn linear_v_returns_expected() {
    const LINEAR: LedLayout<5, 1, 5> = LedLayout::linear_v();
    assert_eq!(
        LINEAR.index_to_xy(),
        &[(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)]
    );
}

#[test]
fn linear_row_major_3x2_matches_expected() {
    const MAP: LedLayout<6, 3, 2> =
        LedLayout::new([(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)]);
    assert_eq!(
        *MAP.index_to_xy(),
        [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1),]
    );
}

#[test]
fn rotate_and_flip_small_grid() {
    const MAP: LedLayout<6, 3, 2> =
        LedLayout::new([(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)]);
    let rotated = MAP.rotate_cw();
    assert_eq!(
        *rotated.index_to_xy(),
        [(1, 0), (1, 1), (1, 2), (0, 0), (0, 1), (0, 2),]
    );

    let flipped = MAP.flip_h();
    assert_eq!(
        *flipped.index_to_xy(),
        [(2, 0), (1, 0), (0, 0), (2, 1), (1, 1), (0, 1),]
    );
}

#[test]
fn serpentine_transforms_match_expected() {
    const SERPENTINE: LedLayout<6, 3, 2> = LedLayout::<6, 3, 2>::serpentine_column_major();

    let rotated_cw = SERPENTINE.rotate_cw();
    assert_eq!(
        *rotated_cw.index_to_xy(),
        [(1, 0), (0, 0), (0, 1), (1, 1), (1, 2), (0, 2),]
    );

    let rotated_180 = SERPENTINE.rotate_180();
    assert_eq!(
        *rotated_180.index_to_xy(),
        [(2, 1), (2, 0), (1, 0), (1, 1), (0, 1), (0, 0),]
    );

    let rotated_ccw = SERPENTINE.rotate_ccw();
    assert_eq!(
        *rotated_ccw.index_to_xy(),
        [(0, 2), (1, 2), (1, 1), (0, 1), (0, 0), (1, 0),]
    );

    let flipped_h = SERPENTINE.flip_h();
    assert_eq!(
        *flipped_h.index_to_xy(),
        [(2, 0), (2, 1), (1, 1), (1, 0), (0, 0), (0, 1),]
    );

    let flipped_v = SERPENTINE.flip_v();
    assert_eq!(
        *flipped_v.index_to_xy(),
        [(0, 1), (0, 0), (1, 0), (1, 1), (2, 1), (2, 0),]
    );

    let combine_h = SERPENTINE.combine_h::<6, 12, 3, 6>(SERPENTINE);
    assert_eq!(
        *combine_h.index_to_xy(),
        [
            (0, 0),
            (0, 1),
            (1, 1),
            (1, 0),
            (2, 0),
            (2, 1),
            (3, 0),
            (3, 1),
            (4, 1),
            (4, 0),
            (5, 0),
            (5, 1),
        ]
    );

    let combine_v = SERPENTINE.combine_v::<6, 12, 2, 4>(SERPENTINE);
    assert_eq!(
        *combine_v.index_to_xy(),
        [
            (0, 0),
            (0, 1),
            (1, 1),
            (1, 0),
            (2, 0),
            (2, 1),
            (0, 2),
            (0, 3),
            (1, 3),
            (1, 2),
            (2, 2),
            (2, 3),
        ]
    );
}

#[test]
fn combine_horizontal_and_vertical() {
    const LEFT: LedLayout<2, 2, 1> = LedLayout::new([(0, 0), (1, 0)]);
    const RIGHT: LedLayout<4, 4, 1> = LedLayout::new([(0, 0), (1, 0), (2, 0), (3, 0)]);
    let combined_h = LEFT.combine_h::<4, 6, 4, 6>(RIGHT);
    assert_eq!(
        combined_h.index_to_xy(),
        &[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0)]
    );

    const TOP: LedLayout<2, 1, 2> = LedLayout::new([(0, 0), (0, 1)]);
    const BOTTOM: LedLayout<3, 1, 3> = LedLayout::new([(0, 0), (0, 1), (0, 2)]);
    let combined_v = TOP.combine_v::<3, 5, 3, 5>(BOTTOM);
    assert_eq!(
        *combined_v.index_to_xy(),
        [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4),]
    );
}

#[test]
#[should_panic(expected = "duplicate (col,row) in mapping")]
fn new_panics_on_duplicate_cell() {
    let _ = LedLayout::<3, 3, 1>::new([(0, 0), (1, 0), (1, 0)]);
}

#[test]
#[should_panic(expected = "column out of bounds")]
fn new_panics_on_out_of_bounds_column() {
    let _ = LedLayout::<3, 3, 1>::new([(0, 0), (1, 0), (3, 0)]);
}

#[test]
#[should_panic(expected = "duplicate (col,row) in mapping")]
fn new_panics_on_missing_cells() {
    // Duplicate causes a cell to be missing; duplicate check fires first.
    let _ = LedLayout::<4, 2, 2>::new([(0, 0), (1, 0), (0, 1), (0, 1)]);
}

#[test]
#[should_panic(expected = "W*H must equal N")]
fn new_panics_on_mismatched_dimensions() {
    let _ = LedLayout::<5, 3, 2>::new([(0, 0), (1, 0), (2, 0), (0, 1), (1, 1)]);
}
