//! Module containing [`LedLayout`], the struct for compile-time description of
//! panel geometry and wiring.
//!
//! See [`LedLayout`] for details and examples.

/// Compile-time description of panel geometry and wiring, including dimensions (with examples).
///
/// `LedLayout` defines how a rectangular `(x, y)` panel of LEDs maps to the linear
/// wiring order of LEDs on a NeoPixel-style (WS2812) panel. It stores both the
/// wiring-order mapping and its inverse, so runtime adapters can borrow the
/// checked inverse from compile-time layout data.
///
/// For examples of `LedLayout` in use, see the [`led2d`](mod@crate::led2d) module,
/// [`Frame2d`](crate::led2d::Frame2d), and the example below.
///
/// **What `LedLayout` does:**
/// - Lets you describe panel wiring once
/// - Enables drawing text, graphics, and animations in `(x, y)` space
/// - Hides LED strip order from rendering code
///
/// Coordinates use a screen-style convention:
/// - `(0, 0)` is the top-left corner
/// - `x` increases to the right
/// - `y` increases downward
///
/// Most users should start with one of the constructors below and then apply
/// transforms ([rotate_cw](`Self::rotate_cw`), [flip_h](`Self::flip_h`), [combine_v](`Self::combine_v`), etc.)
/// as needed.
///
/// ## Constructing layouts
///
/// Prefer the built-in constructors when possible:
/// - [`serpentine_row_major`](Self::serpentine_row_major)
/// - [`serpentine_column_major`](Self::serpentine_column_major)
/// - [`linear_h`](Self::linear_h) / [`linear_v`](Self::linear_v)
///
/// For unusual wiring, you can construct a layout directly with [`LedLayout::new`]
/// by listing `(x, y)` for each LED in the order the strip is wired.
///
/// **The example below shows both construction methods.** Also, the documentation for every constructor
/// and method includes illustrations of use.
///
/// ## Transforming layouts
///
/// You can adapt a layout without rewriting it:
/// - rotate: [`rotate_cw`](Self::rotate_cw), [`rotate_ccw`](Self::rotate_ccw), [`rotate_180`](Self::rotate_180)
/// - flip: [`flip_h`](Self::flip_h), [`flip_v`](Self::flip_v)
/// - combine: [`combine_h`](Self::combine_h), [`combine_v`](Self::combine_v)  (join two layouts into a larger one)
///
/// ## Validation
///
/// Layouts are validated at **compile time**:
/// - coordinates must be in-bounds
/// - every `(x, y)` cell must appear exactly once
///
/// The [`new`](Self::new) constructor proves that the mapping is one-to-one and
/// constructs both directions. The stored directions are:
///
/// ```text
/// index_to_xy[physical_led_index] = (x, y)
/// xy_to_index[y * W + x] = physical_led_index
/// ```
///
/// Drawing uses [`xy_to_index`](Self::xy_to_index), while layout composition,
/// transformations, and inspection use [`index_to_xy`](Self::index_to_xy).
///
/// # Example
///
/// Rotate a serpentine-wired 3×2 panel into a 2×3 layout and verify the result at compile time:
///
/// ```rust,no_run
/// use device_envoy_core::led2d::layout::LedLayout;
///
/// const ROTATED: LedLayout<6, 2, 3> = LedLayout::serpentine_column_major().rotate_cw();
/// const EXPECTED: LedLayout<6, 2, 3> =
///     LedLayout::new([(1, 0), (0, 0), (0, 1), (1, 1), (1, 2), (0, 2)]);
/// const _: () = assert!(ROTATED.equals(&EXPECTED)); // Compile-time assert
/// ```
///
/// ```text
/// Serpentine 3×2 rotated to 2×3:
///
///   Before:              After:
///     LED0  LED3  LED4     LED1  LED0
///     LED1  LED2  LED5     LED2  LED3
///                          LED5  LED4
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LedLayout<const N: usize, const W: usize, const H: usize> {
    index_to_xy: [(u16, u16); N],
    xy_to_index: [u16; N],
}

impl<const N: usize, const W: usize, const H: usize> LedLayout<N, W, H> {
    /// Return the array mapping LED wiring order to `(x, y)` coordinates.
    #[must_use]
    pub const fn index_to_xy(&self) -> &[(u16, u16); N] {
        &self.index_to_xy
    }

    /// The width of the layout.
    #[must_use]
    pub const fn width(&self) -> usize {
        W
    }

    /// The height of the layout.
    #[must_use]
    pub const fn height(&self) -> usize {
        H
    }

    /// Total LEDs in this layout (width × height).
    #[must_use]
    pub const fn len(&self) -> usize {
        N
    }

    /// Return the borrowed inverse mapping from `(x, y)` coordinates to LED wiring index.
    ///
    /// The array directions are `index_to_xy[physical_led_index] = (x, y)` and
    /// `xy_to_index[y * W + x] = physical_led_index`. See the
    /// [`LedLayout`] example for both directions in use.
    #[must_use]
    pub const fn xy_to_index(&self) -> &[u16; N] {
        &self.xy_to_index
    }

    /// Const equality helper for doctests/examples.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::led2d::layout::LedLayout;
    ///
    /// const LINEAR: LedLayout<4, 4, 1> = LedLayout::linear_h();
    /// const ROTATED: LedLayout<4, 4, 1> = LedLayout::linear_v().rotate_cw();
    ///
    /// const _: () = assert!(LINEAR.equals(&LINEAR));   // assert equal
    /// const _: () = assert!(!LINEAR.equals(&ROTATED)); // assert not equal
    /// ```
    ///
    /// ```text
    /// LINEAR:  LED0  LED1  LED2  LED3
    /// ROTATED: LED3  LED2  LED1  LED0
    /// ```
    #[must_use]
    pub const fn equals(&self, other: &Self) -> bool {
        let mut i = 0;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
        while i < N {
            if self.index_to_xy[i].0 != other.index_to_xy[i].0
                || self.index_to_xy[i].1 != other.index_to_xy[i].1
            {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Construct a `LedLayout` by explicitly specifying the wiring order.
    ///
    /// Use this constructor when your panel wiring does not match one of the
    /// built-in patterns (linear, serpentine, etc.). You provide the `(x, y)`
    /// coordinate for **each LED in strip order**, and `LedLayout` derives the
    /// inverse mapping from it.
    ///
    /// This constructor is `const` and is intended to be used in a `const`
    /// definition, so layout errors are caught at **compile time**, not at runtime.
    /// ```rust,no_run
    /// use device_envoy_core::led2d::layout::LedLayout;
    ///
    /// // 3×2 panel (landscape, W×H)
    /// const MAP: LedLayout<6, 3, 2> =
    ///     LedLayout::new([(0, 0), (1, 0), (2, 0), (2, 1), (1, 1), (0, 1)]);
    ///
    /// // Rotate to portrait (CW)
    /// const ROTATED: LedLayout<6, 2, 3> = MAP.rotate_cw();
    ///
    /// // Expected: 2×3 panel (W×H)
    /// const EXPECTED: LedLayout<6, 2, 3> =
    ///     LedLayout::new([(1, 0), (1, 1), (1, 2), (0, 2), (0, 1), (0, 0)]);
    ///
    /// const _: () = assert!(ROTATED.equals(&EXPECTED));
    /// ```
    ///
    /// ```text
    /// 3×2 input (col,row by LED index):
    ///   LED0  LED1  LED2
    ///   LED5  LED4  LED3
    ///
    /// After rotate to 2×3:
    ///   LED1  LED0
    ///   LED2  LED3
    ///   LED5  LED4
    /// ```
    #[must_use]
    pub const fn new(index_to_xy: [(u16, u16); N]) -> Self {
        // TODO Consider allowing zero-sized layouts as identity values for composition.
        assert!(W > 0 && H > 0, "W and H must be positive");
        assert!(W * H == N, "W*H must equal N");
        assert!(N <= u16::MAX as usize, "total LEDs must fit in u16");

        let mut seen = [false; N];
        let mut xy_to_index = [0_u16; N];

        let mut led_index = 0;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
        while led_index < N {
            let (x, y) = index_to_xy[led_index];
            let x = x as usize;
            let y = y as usize;

            assert!(x < W, "column out of bounds");
            assert!(y < H, "row out of bounds");

            let cell_index = y * W + x;
            assert!(!seen[cell_index], "duplicate (col,row) in mapping");
            seen[cell_index] = true;
            xy_to_index[cell_index] = led_index as u16;

            led_index += 1;
        }

        let mut k = 0;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
        while k < N {
            assert!(seen[k], "mapping does not cover every cell");
            k += 1;
        }

        Self {
            index_to_xy,
            xy_to_index,
        }
    }

    /// Linear row-major mapping for a single-row strip (cols increase left-to-right).
    ///
    /// ```rust,no_run
    /// use device_envoy_core::led2d::layout::LedLayout;
    ///
    /// const LINEAR: LedLayout<6, 6, 1> = LedLayout::linear_h();
    /// const EXPECTED: LedLayout<6, 6, 1> =
    ///     LedLayout::new([(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0)]);
    /// const _: () = assert!(LINEAR.equals(&EXPECTED));
    /// ```
    ///
    /// ```text
    /// 6×1 strip maps to single row:
    ///   LED0  LED1  LED2  LED3  LED4  LED5
    /// ```
    #[must_use]
    pub const fn linear_h() -> Self {
        assert!(H == 1, "linear_h requires H == 1");
        assert!(W == N, "linear_h requires W == N");

        let mut mapping = [(0_u16, 0_u16); N];
        let mut x_index = 0;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
        while x_index < W {
            mapping[x_index] = (x_index as u16, 0);
            x_index += 1;
        }
        Self::new(mapping)
    }

    /// Linear column-major mapping for a single-column strip (rows increase top-to-bottom).
    ///
    /// ```rust,no_run
    /// use device_envoy_core::led2d::layout::LedLayout;
    ///
    /// const LINEAR: LedLayout<6, 1, 6> = LedLayout::linear_v();
    /// const EXPECTED: LedLayout<6, 1, 6> =
    ///     LedLayout::new([(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]);
    /// const _: () = assert!(LINEAR.equals(&EXPECTED));
    /// ```
    ///
    /// ```text
    /// 1×6 strip maps to single column:
    ///   LED0
    ///   LED1
    ///   LED2
    ///   LED3
    ///   LED4
    ///   LED5
    /// ```
    #[must_use]
    pub const fn linear_v() -> Self {
        assert!(W == 1, "linear_v requires W == 1");
        assert!(H == N, "linear_v requires H == N");

        let mut mapping = [(0_u16, 0_u16); N];
        let mut y_index = 0;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
        while y_index < H {
            mapping[y_index] = (0, y_index as u16);
            y_index += 1;
        }
        Self::new(mapping)
    }

    /// Serpentine column-major mapping returned as a checked `LedLayout`.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::led2d::layout::LedLayout;
    ///
    /// const MAP: LedLayout<6, 3, 2> = LedLayout::serpentine_column_major();
    /// const EXPECTED: LedLayout<6, 3, 2> =
    ///     LedLayout::new([(0, 0), (0, 1), (1, 1), (1, 0), (2, 0), (2, 1)]);
    /// const _: () = assert!(MAP.equals(&EXPECTED));
    /// ```
    ///
    /// ```text
    /// Strip snakes down columns (3×2 example):
    ///   LED0  LED3  LED4
    ///   LED1  LED2  LED5
    /// ```
    #[must_use]
    pub const fn serpentine_column_major() -> Self {
        assert!(W > 0 && H > 0, "W and H must be positive");
        assert!(W * H == N, "W*H must equal N");

        let mut mapping = [(0_u16, 0_u16); N];
        let mut y_index = 0;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace these while loops with for loops.
        while y_index < H {
            let mut x_index = 0;
            while x_index < W {
                let led_index = if x_index % 2 == 0 {
                    // Even column: top-to-bottom
                    x_index * H + y_index
                } else {
                    // Odd column: bottom-to-top
                    x_index * H + (H - 1 - y_index)
                };
                mapping[led_index] = (x_index as u16, y_index as u16);
                x_index += 1;
            }
            y_index += 1;
        }
        Self::new(mapping)
    }

    /// Serpentine row-major mapping (alternating left-to-right and right-to-left across rows).
    ///
    /// ```rust,no_run
    /// use device_envoy_core::led2d::layout::LedLayout;
    ///
    /// const MAP: LedLayout<6, 3, 2> = LedLayout::serpentine_row_major();
    /// const EXPECTED: LedLayout<6, 3, 2> =
    ///     LedLayout::new([(0, 0), (1, 0), (2, 0), (2, 1), (1, 1), (0, 1)]);
    /// const _: () = assert!(MAP.equals(&EXPECTED));
    /// ```
    ///
    /// ```text
    /// Strip snakes across rows (3×2 example):
    ///   LED0  LED1  LED2
    ///   LED5  LED4  LED3
    /// ```
    #[must_use]
    pub const fn serpentine_row_major() -> Self {
        assert!(W > 0 && H > 0, "W and H must be positive");
        assert!(W * H == N, "W*H must equal N");

        let mut mapping = [(0_u16, 0_u16); N];
        let mut y_index = 0;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace these while loops with for loops.
        while y_index < H {
            let mut x_index = 0;
            while x_index < W {
                let led_index = if y_index % 2 == 0 {
                    y_index * W + x_index
                } else {
                    y_index * W + (W - 1 - x_index)
                };
                mapping[led_index] = (x_index as u16, y_index as u16);
                x_index += 1;
            }
            y_index += 1;
        }
        Self::new(mapping)
    }

    /// Rotate 90° clockwise (dims swap).
    ///
    /// ```rust,no_run
    /// use device_envoy_core::led2d::layout::LedLayout;
    ///
    /// const ROTATED: LedLayout<6, 2, 3> = LedLayout::serpentine_column_major().rotate_cw();
    /// const EXPECTED: LedLayout<6, 2, 3> =
    ///     LedLayout::new([(1, 0), (0, 0), (0, 1), (1, 1), (1, 2), (0, 2)]);
    /// const _: () = assert!(ROTATED.equals(&EXPECTED));
    /// ```
    ///
    /// ```text
    /// Before (3×2 serpentine): After (2×3):
    ///   LED0  LED3  LED4        LED1  LED0
    ///   LED1  LED2  LED5        LED2  LED3
    ///                           LED5  LED4
    /// ```
    #[must_use]
    pub const fn rotate_cw(self) -> LedLayout<N, H, W> {
        let mut out = [(0u16, 0u16); N];
        let mut i = 0;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
        while i < N {
            let (c, r) = self.index_to_xy[i];
            let c = c as usize;
            let r = r as usize;
            out[i] = ((H - 1 - r) as u16, c as u16);
            i += 1;
        }
        LedLayout::<N, H, W>::new(out)
    }

    /// Flip horizontally (mirror columns).
    ///
    /// ```rust,no_run
    /// use device_envoy_core::led2d::layout::LedLayout;
    ///
    /// const FLIPPED: LedLayout<6, 3, 2> = LedLayout::serpentine_column_major().flip_h();
    /// const EXPECTED: LedLayout<6, 3, 2> =
    ///     LedLayout::new([(2, 0), (2, 1), (1, 1), (1, 0), (0, 0), (0, 1)]);
    /// const _: () = assert!(FLIPPED.equals(&EXPECTED));
    /// ```
    ///
    /// ```text
    /// Before (serpentine): After:
    ///   LED0  LED3  LED4      LED4  LED3  LED0
    ///   LED1  LED2  LED5      LED5  LED2  LED1
    /// ```
    #[must_use]
    pub const fn flip_h(self) -> Self {
        let mut out = [(0u16, 0u16); N];
        let mut i = 0;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
        while i < N {
            let (c, r) = self.index_to_xy[i];
            let c = c as usize;
            out[i] = ((W - 1 - c) as u16, r);
            i += 1;
        }
        Self::new(out)
    }

    /// Rotate 180° derived from rotate_cw.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::led2d::layout::LedLayout;
    ///
    /// const ROTATED: LedLayout<6, 3, 2> = LedLayout::serpentine_column_major().rotate_180();
    /// const EXPECTED: LedLayout<6, 3, 2> =
    ///     LedLayout::new([(2, 1), (2, 0), (1, 0), (1, 1), (0, 1), (0, 0)]);
    /// const _: () = assert!(ROTATED.equals(&EXPECTED));
    /// ```
    ///
    /// ```text
    /// Before (3×2 serpentine): After 180°:
    ///   LED0  LED3  LED4        LED5  LED2  LED1
    ///   LED1  LED2  LED5        LED4  LED3  LED0
    /// ```
    #[must_use]
    pub const fn rotate_180(self) -> Self {
        self.rotate_cw().rotate_cw()
    }

    /// Rotate 90° counter-clockwise derived from rotate_cw.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::led2d::layout::LedLayout;
    ///
    /// const ROTATED: LedLayout<6, 2, 3> = LedLayout::serpentine_column_major().rotate_ccw();
    /// const EXPECTED: LedLayout<6, 2, 3> =
    ///     LedLayout::new([(0, 2), (1, 2), (1, 1), (0, 1), (0, 0), (1, 0)]);
    /// const _: () = assert!(ROTATED.equals(&EXPECTED));
    /// ```
    ///
    /// ```text
    /// Before (3×2 serpentine): After (2×3):
    ///   LED0  LED3  LED4        LED4  LED5
    ///   LED1  LED2  LED5        LED3  LED2
    ///                           LED0  LED1
    /// ```
    #[must_use]
    pub const fn rotate_ccw(self) -> LedLayout<N, H, W> {
        self.rotate_cw().rotate_cw().rotate_cw()
    }

    /// Flip vertically derived from rotation + horizontal flip.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::led2d::layout::LedLayout;
    ///
    /// const FLIPPED: LedLayout<6, 3, 2> = LedLayout::serpentine_column_major().flip_v();
    /// const EXPECTED: LedLayout<6, 3, 2> =
    ///     LedLayout::new([(0, 1), (0, 0), (1, 0), (1, 1), (2, 1), (2, 0)]);
    /// const _: () = assert!(FLIPPED.equals(&EXPECTED));
    /// ```
    ///
    /// ```text
    /// Before (serpentine): After:
    ///   LED0  LED3  LED4      LED1  LED2  LED5
    ///   LED1  LED2  LED5      LED0  LED3  LED4
    /// ```
    #[must_use]
    pub const fn flip_v(self) -> Self {
        self.rotate_cw().flip_h().rotate_ccw()
    }

    /// Concatenate horizontally with another mapping sharing the same rows.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::led2d::layout::LedLayout;
    ///
    /// const LED_LAYOUT: LedLayout<6, 3, 2> = LedLayout::serpentine_column_major();
    /// const COMBINED: LedLayout<12, 6, 2> = LED_LAYOUT.combine_h::<6, 12, 3, 6>(LED_LAYOUT);
    /// const EXPECTED: LedLayout<12, 6, 2> = LedLayout::new([
    ///     (0, 0), (0, 1), (1, 1), (1, 0), (2, 0), (2, 1), (3, 0), (3, 1), (4, 1),
    ///     (4, 0), (5, 0), (5, 1),
    /// ]);
    /// const _: () = assert!(COMBINED.equals(&EXPECTED));
    /// ```
    ///
    /// ```text
    /// Left serpentine (3×2):    Right serpentine (3×2):
    ///   0  3  4                   6  9 10
    ///   1  2  5                   7  8 11
    ///
    /// Combined (6×2):
    ///   0  3  4  6  9 10
    ///   1  2  5  7  8 11
    /// ```
    #[must_use]
    pub const fn combine_h<
        const N2: usize,
        const OUT_N: usize,
        const W2: usize,
        const OUT_W: usize,
    >(
        self,
        right: LedLayout<N2, W2, H>,
    ) -> LedLayout<OUT_N, OUT_W, H> {
        assert!(OUT_N == N + N2, "OUT_N must equal LEFT + RIGHT");
        assert!(OUT_W == W + W2, "OUT_W must equal W + W2");

        let mut out = [(0u16, 0u16); OUT_N];

        let mut i = 0;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace these while loops with for loops.
        while i < N {
            out[i] = self.index_to_xy[i];
            i += 1;
        }

        let mut j = 0;
        while j < N2 {
            let (c, r) = right.index_to_xy[j];
            out[N + j] = ((c as usize + W) as u16, r);
            j += 1;
        }

        LedLayout::<OUT_N, OUT_W, H>::new(out)
    }

    /// Concatenate vertically with another mapping sharing the same columns.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::led2d::layout::LedLayout;
    ///
    /// const LED_LAYOUT: LedLayout<6, 3, 2> = LedLayout::serpentine_column_major();
    /// const COMBINED: LedLayout<12, 3, 4> = LED_LAYOUT.combine_v::<6, 12, 2, 4>(LED_LAYOUT);
    /// const EXPECTED: LedLayout<12, 3, 4> = LedLayout::new([
    ///     (0, 0), (0, 1), (1, 1), (1, 0), (2, 0), (2, 1), (0, 2), (0, 3), (1, 3),
    ///     (1, 2), (2, 2), (2, 3),
    /// ]);
    /// const _: () = assert!(COMBINED.equals(&EXPECTED));
    /// ```
    ///
    /// ```text
    /// Top serpentine (3×2):    Bottom serpentine (3×2):
    ///   0  3  4                   6  9 10
    ///   1  2  5                   7  8 11
    ///
    /// Combined (3×4):
    ///   0  3  4
    ///   1  2  5
    ///   6  9 10
    ///   7  8 11
    /// ```
    #[must_use]
    pub const fn combine_v<
        const N2: usize,
        const OUT_N: usize,
        const H2: usize,
        const OUT_H: usize,
    >(
        self,
        bottom: LedLayout<N2, W, H2>,
    ) -> LedLayout<OUT_N, W, OUT_H> {
        assert!(OUT_N == N + N2, "OUT_N must equal TOP + BOTTOM");
        assert!(OUT_H == H + H2, "OUT_H must equal H + H2");

        // Derive vertical concat via transpose + horizontal concat + transpose back.
        // Transpose is implemented as rotate_cw + flip_h.
        let top_t = self.rotate_cw().flip_h(); // H width, W height
        let bot_t = bottom.rotate_cw().flip_h(); // H2 width, W height

        let combined_t: LedLayout<OUT_N, OUT_H, W> = top_t.combine_h::<N2, OUT_N, H2, OUT_H>(bot_t);

        combined_t.rotate_cw().flip_h() // transpose back to W x OUT_H
    }
}
