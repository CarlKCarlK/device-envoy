use device_envoy_esp32::led2d::layout::LedLayout;

const INVALID: LedLayout<3, 3, 1> = LedLayout::new([(0, 0), (1, 0), (1, 0)]);

fn main() {
    let _invalid = INVALID;
}
