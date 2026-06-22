use ledist_pi::{DisplayBackend, NullBackend, RgbFrame};

#[test]
fn frame_blits_only_inside_the_target_region() {
    let mut frame = RgbFrame::black(4, 2);
    frame
        .blit_rgb(-1, 0, 2, 1, &[255, 0, 0, 0, 255, 0])
        .unwrap();
    assert_eq!(frame.pixel(0, 0), Some([0, 255, 0]));
    assert_eq!(frame.pixel(1, 0), Some([0, 0, 0]));
}

#[test]
fn clearing_region_does_not_touch_other_pixels() {
    let mut frame = RgbFrame::solid(4, 2, [7, 8, 9]);
    frame.clear_region(1, 0, 2, 2);
    assert_eq!(frame.pixel(0, 0), Some([7, 8, 9]));
    assert_eq!(frame.pixel(1, 0), Some([0, 0, 0]));
    assert_eq!(frame.pixel(3, 1), Some([7, 8, 9]));
}

#[test]
fn null_backend_keeps_the_last_presented_frame() {
    let frame = RgbFrame::solid(2, 1, [1, 2, 3]);
    let mut backend = NullBackend::default();
    backend.present(&frame).unwrap();
    assert_eq!(backend.last_frame().unwrap().as_rgb(), frame.as_rgb());
}
