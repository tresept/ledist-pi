use ledist_pi::{AssetRegistry, Profile};
use std::fs;

#[test]
fn profile_rejects_regions_outside_canvas() {
    let text = r#"[profile]
id = "bad"
name = "Bad"
canvas_width = 128
canvas_height = 32

[regions.too_far]
x = 127
y = 0
width = 2
height = 1
"#;
    assert!(
        Profile::from_toml(text)
            .unwrap_err()
            .to_string()
            .contains("too_far")
    );
}

#[test]
fn asset_validation_requires_exact_target_size() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("assets/service/ja")).unwrap();
    let image = image::RgbImage::new(2, 1);
    image
        .save(directory.path().join("assets/service/ja/local.png"))
        .unwrap();
    let registry = AssetRegistry::scan(directory.path()).unwrap();
    assert!(
        registry
            .validate_size("assets/service/ja", "local", 2, 1)
            .is_ok()
    );
    assert!(
        registry
            .validate_size("assets/service/ja", "local", 48, 32)
            .unwrap_err()
            .to_string()
            .contains("expected 48x32")
    );
}

#[test]
fn registry_uses_registered_asset_ids_instead_of_request_paths() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("assets/service/ja")).unwrap();
    fs::write(directory.path().join("assets/service/ja/各駅停車.png"), []).unwrap();
    fs::write(directory.path().join("assets/service/ja/.hidden.png"), []).unwrap();
    let registry = AssetRegistry::scan(directory.path()).unwrap();
    assert!(registry.resolve("assets/service/ja", "各駅停車").is_some());
    assert!(
        registry
            .resolve("assets/service/ja", "../../secret")
            .is_none()
    );
    assert!(registry.resolve("assets/service/ja", ".hidden").is_none());
}
