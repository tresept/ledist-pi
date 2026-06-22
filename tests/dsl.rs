use ledist_pi::{Command, parse_program};

#[test]
fn parses_atomic_frame_and_field_duration() {
    let program =
        parse_program("frame\n set service service_ja\n clear right\nend\nwait ${ja_duration}\n")
            .unwrap();
    assert!(matches!(program.commands[0], Command::Frame(_)));
    assert!(matches!(program.commands[1], Command::WaitField(ref id) if id == "ja_duration"));
}

#[test]
fn rejects_unclosed_nested_loop_with_a_line_number() {
    let error = parse_program("loop\n  frame\n    clear right\n  end")
        .unwrap_err()
        .to_string();
    assert!(error.contains("1行目"), "{error}");
}

#[test]
fn rejects_brightness_outside_the_hardware_range() {
    assert!(parse_program("brightness 101").is_err());
}

#[test]
fn parses_a_counted_loop() {
    let program = parse_program("loop 3\n blank\nend").unwrap();
    assert!(matches!(program.commands[0], Command::Loop(Some(3), _)));
}

#[test]
fn parses_nested_loop_for_scroll_duration() {
    let program = parse_program(
        "loop\n scroll right_bottom scroll_text\n loop\n  wait 3s\n end\n wait scroll_end\nend",
    )
    .unwrap();
    assert!(matches!(program.commands[0], Command::Loop(None, _)));
}

#[test]
fn parses_while_scroll_and_check_scroll() {
    let program = parse_program("while scroll\n wait 3s\n check scroll\nend").unwrap();
    assert!(matches!(program.commands[0], Command::WhileScroll(_)));
}

#[test]
fn reports_line_for_unknown_statement() {
    assert!(
        parse_program("wat 3s")
            .unwrap_err()
            .to_string()
            .contains("1行目")
    );
}
