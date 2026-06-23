use ledist_pi::Profile;

#[test]
fn e233_9000_defines_every_common_and_optional_asset_variant() {
    let profile =
        Profile::from_toml(include_str!("../data/trains/e233-9000/profile.toml")).unwrap();
    let assets = &profile.e233.unwrap().assets;
    for (group, variants) in [
        ("service", &["full", "left-ja", "left-en"][..]),
        (
            "destination",
            &["full", "right", "right-top-ja", "right-top-en", "full-top"][..],
        ),
        ("route", &["right", "right-top"][..]),
        ("through_route", &["right", "right-top"][..]),
        ("service_change", &["right"][..]),
        (
            "next_stop",
            &[
                "right-bottom-ja",
                "right-bottom-en",
                "full-bottom-ja",
                "full-bottom-en",
            ][..],
        ),
    ] {
        let configured = &assets[group].directories;
        for variant in variants {
            assert!(configured.contains_key(*variant), "{group}/{variant}");
        }
    }
}
