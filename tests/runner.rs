use ledist_pi::{FrameRunner, RgbFrame, parse_program};

#[test]
fn frame_operations_are_applied_to_a_single_next_frame() {
    let program = parse_program("frame\n set left red\n set right green\nend").unwrap();
    let mut runner = FrameRunner::new(4, 1);
    runner
        .apply_first_frame(&program, |field| match field {
            "red" => Some((0, 0, 2, 1, vec![255, 0, 0, 255, 0, 0])),
            "green" => Some((2, 0, 2, 1, vec![0, 255, 0, 0, 255, 0])),
            _ => None,
        })
        .unwrap();
    let mut expected = RgbFrame::solid(4, 1, [0, 255, 0]);
    expected
        .blit_rgb(0, 0, 2, 1, &[255, 0, 0, 255, 0, 0])
        .unwrap();
    assert_eq!(runner.frame(), &expected);
}
