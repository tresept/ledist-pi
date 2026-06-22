use ledist_pi::{BdfFont, RgbFrame, RuntimeConfig};

#[test]
fn runtime_config_parses_matrix_settings_and_rejects_invalid_brightness() {
    let config = RuntimeConfig::from_toml(
        r#"
backend = "simulator"
brightness = 40
[matrix]
rows = 32
cols = 64
chain_length = 2
rp1_backend = "rio"
"#,
    )
    .unwrap();
    assert_eq!(config.matrix.canvas_size(), (128, 32));
    assert!(RuntimeConfig::from_toml("backend='null'\nbrightness=101").is_err());
}

#[test]
fn bdf_font_draws_a_unicode_glyph_and_rejects_missing_one() {
    let font = BdfFont::parse_bdf(
        "STARTFONT 2.1\nFONTBOUNDINGBOX 8 1 0 0\nSTARTCHAR A\nENCODING 65\nDWIDTH 8 0\nBBX 8 1 0 0\nBITMAP\n80\nENDCHAR\nENDFONT\n",
    )
    .unwrap();
    assert_eq!(font.measure("A"), 8);
    assert!(font.measure_checked("A").is_ok());
    assert!(font.measure_checked("あ").is_err());
    let mut frame = RgbFrame::black(8, 1);
    font.draw("A", &mut frame, 0, 0, [1, 2, 3]).unwrap();
    assert_eq!(frame.pixel(0, 0), Some([1, 2, 3]));
}
