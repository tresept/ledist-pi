use ledist_pi::{
    AssetRegistry, E233DisplaySelection, E233Layout, FieldSelection, Profile, ScriptEvent,
    ScrollCycleItem, compile_e233, plan_e233,
};
use std::{
    fs,
    time::{Duration, Instant},
};

fn selection() -> E233DisplaySelection {
    E233DisplaySelection {
        service: FieldSelection::None,
        route: FieldSelection::None,
        service_change: FieldSelection::None,
        through_route: FieldSelection::None,
        destination: FieldSelection::None,
        next_stop: FieldSelection::None,
        scroll_text: String::new(),
        scroll_speed: 50.0,
        scroll_cycle: vec![
            ScrollCycleItem::DestinationJa,
            ScrollCycleItem::DestinationEn,
        ],
        brightness: 40,
    }
}
#[test]
fn destination_and_service_change_are_separate_pages() {
    let mut s = selection();
    s.service = FieldSelection::Asset("s".into());
    s.destination = FieldSelection::Asset("d".into());
    s.service_change = FieldSelection::Asset("c".into());
    assert_eq!(plan_e233(&s).unwrap().pages.len(), 2)
}
#[test]
fn makes_service_destination_scroll_page() {
    let mut s = selection();
    s.service = FieldSelection::Asset("s".into());
    s.destination = FieldSelection::Asset("d".into());
    s.scroll_text = "next".into();
    let plan = plan_e233(&s).unwrap();
    assert!(matches!(
        plan.pages.last().unwrap().layout,
        E233Layout::ServiceAndRightSplit(_, _, _)
    ));
}

#[test]
fn next_stop_creates_japanese_and_english_split_pages() {
    let mut s = selection();
    s.service = FieldSelection::Asset("local".into());
    s.destination = FieldSelection::Asset("shinjuku".into());
    s.next_stop = FieldSelection::Asset("osaki".into());
    let plan = plan_e233(&s).unwrap();
    assert_eq!(plan.pages.len(), 2);
    assert!(matches!(
        plan.pages[0].layout,
        E233Layout::ServiceAndRightSplit(..)
    ));
    assert!(matches!(
        plan.pages[1].layout,
        E233Layout::ServiceAndRightSplit(..)
    ));
}

#[test]
fn scroll_requires_destination() {
    let selection = E233DisplaySelection {
        service: FieldSelection::Asset("local".into()),
        route: FieldSelection::Asset("saikyo".into()),
        service_change: FieldSelection::None,
        through_route: FieldSelection::None,
        destination: FieldSelection::None,
        next_stop: FieldSelection::None,
        scroll_text: "この電車は相鉄線へ直通します".into(),
        scroll_speed: 50.0,
        scroll_cycle: vec![ScrollCycleItem::DestinationJa],
        brightness: 40,
    };

    assert!(plan_e233(&selection).is_err());
}

#[test]
fn static_pages_follow_destination_route_through_change_order() {
    let mut s = selection();
    s.service = FieldSelection::Blank;
    s.destination = FieldSelection::Asset("d".into());
    s.route = FieldSelection::Asset("r".into());
    s.through_route = FieldSelection::Asset("t".into());
    let plan = plan_e233(&s).unwrap();
    assert_eq!(plan.pages.len(), 2);
    assert!(matches!(
        plan.pages[0].layout,
        E233Layout::ServiceAndRight(..)
    ));
    assert!(matches!(
        plan.pages[1].layout,
        E233Layout::ServiceAndRightSplit(..)
    ));
}

#[test]
fn route_and_through_are_composed_into_one_static_page() {
    let mut s = selection();
    s.service = FieldSelection::Asset("local".into());
    s.route = FieldSelection::Asset("saikyo".into());
    s.through_route = FieldSelection::Asset("sotetsu".into());
    let plan = plan_e233(&s).unwrap();
    assert_eq!(plan.pages.len(), 1);
    assert!(matches!(
        plan.pages[0].layout,
        E233Layout::ServiceAndRightSplit(..)
    ));
}

#[test]
fn rejects_scroll_without_a_cycle_item_or_destination() {
    let mut s = selection();
    s.destination = FieldSelection::Asset("d".into());
    s.scroll_text = "notice".into();
    s.scroll_cycle.clear();
    assert!(plan_e233(&s).is_err());
    let mut s = selection();
    s.scroll_text = "notice".into();
    assert!(plan_e233(&s).is_err());
}

#[test]
fn service_with_only_a_128_by_32_asset_is_black_in_a_composite_page() {
    let root = tempfile::tempdir().unwrap();
    let train = root.path().join("train");
    fs::create_dir_all(train.join("assets/service/128x32")).unwrap();
    fs::create_dir_all(train.join("assets/destination/80x32")).unwrap();
    image::RgbImage::from_pixel(128, 32, image::Rgb([9, 8, 7]))
        .save(train.join("assets/service/128x32/out_of_service.png"))
        .unwrap();
    image::RgbImage::from_pixel(80, 32, image::Rgb([1, 2, 3]))
        .save(train.join("assets/destination/80x32/shinjuku.png"))
        .unwrap();
    let profile = Profile::from_toml(
        r#"
[profile]
id='e233'
name='E233'
[e233]
[e233.assets.service]
label='種別'
[e233.assets.service.directories]
full=['assets/service/128x32']
left-ja=['assets/service/48x32/ja']
left-en=['assets/service/48x32/en']
[e233.assets.destination]
label='行先'
[e233.assets.destination.directories]
right=['assets/destination/80x32']
"#,
    )
    .unwrap();
    let mut s = selection();
    s.service = FieldSelection::Asset("out_of_service".into());
    s.destination = FieldSelection::Asset("shinjuku".into());
    let mut runner = compile_e233(
        &profile,
        &AssetRegistry::scan(&train).unwrap(),
        &s,
        root.path(),
    )
    .unwrap();
    let events = runner.tick(Instant::now()).unwrap();
    let ScriptEvent::Present(frame) = &events[0] else {
        panic!("expected frame")
    };
    assert_eq!(frame.pixel(0, 0), Some([0, 0, 0]));
    assert_eq!(frame.pixel(48, 0), Some([1, 2, 3]));
}

#[test]
fn destination_without_service_or_scroll_uses_the_full_destination_curtain() {
    let root = tempfile::tempdir().unwrap();
    let train = root.path().join("train");
    fs::create_dir_all(train.join("assets/destination/128x32")).unwrap();
    image::RgbImage::from_pixel(128, 32, image::Rgb([4, 5, 6]))
        .save(train.join("assets/destination/128x32/shinjuku.png"))
        .unwrap();
    let profile = Profile::from_toml(
        r#"
[profile]
id='e233'
name='E233'
[e233]
[e233.assets.destination]
label='行先'
[e233.assets.destination.directories]
full=['assets/destination/128x32']
full-top=['assets/destination/128x16']
"#,
    )
    .unwrap();
    let mut s = selection();
    s.destination = FieldSelection::Asset("shinjuku".into());
    let mut runner = compile_e233(
        &profile,
        &AssetRegistry::scan(&train).unwrap(),
        &s,
        root.path(),
    )
    .unwrap();
    let events = runner.tick(Instant::now()).unwrap();
    let ScriptEvent::Present(frame) = &events[0] else {
        panic!("expected frame")
    };
    assert_eq!(frame.pixel(0, 0), Some([4, 5, 6]));
}

#[test]
fn destination_without_a_full_asset_falls_back_to_the_right_80_by_32_asset() {
    let root = tempfile::tempdir().unwrap();
    let train = root.path().join("train");
    fs::create_dir_all(train.join("assets/destination/80x32")).unwrap();
    image::RgbImage::from_pixel(80, 32, image::Rgb([2, 4, 6]))
        .save(train.join("assets/destination/80x32/shinjuku.png"))
        .unwrap();
    let profile = Profile::from_toml(
        r#"
[profile]
id='e233'
name='E233'
[e233]
[e233.assets.destination]
label='行先'
[e233.assets.destination.directories]
full=['assets/destination/128x32']
right=['assets/destination/80x32']
right-top-ja=['assets/destination/80x16/ja']
"#,
    )
    .unwrap();
    let mut s = selection();
    s.destination = FieldSelection::Asset("shinjuku".into());
    let mut runner = compile_e233(
        &profile,
        &AssetRegistry::scan(&train).unwrap(),
        &s,
        root.path(),
    )
    .unwrap();
    let events = runner.tick(Instant::now()).unwrap();
    let ScriptEvent::Present(frame) = &events[0] else {
        panic!("expected frame")
    };
    assert_eq!(frame.pixel(0, 0), Some([0, 0, 0]));
    assert_eq!(frame.pixel(48, 0), Some([2, 4, 6]));
}

#[test]
fn destination_without_full_or_right_asset_falls_back_to_the_right_top_asset() {
    let root = tempfile::tempdir().unwrap();
    let train = root.path().join("train");
    fs::create_dir_all(train.join("assets/destination/80x16/ja")).unwrap();
    image::RgbImage::from_pixel(80, 16, image::Rgb([3, 6, 9]))
        .save(train.join("assets/destination/80x16/ja/shinjuku.png"))
        .unwrap();
    let profile = Profile::from_toml(
        r#"
[profile]
id='e233'
name='E233'
[e233]
[e233.assets.destination]
label='行先'
[e233.assets.destination.directories]
full=['assets/destination/128x32']
right=['assets/destination/80x32']
right-top-ja=['assets/destination/80x16/ja']
"#,
    )
    .unwrap();
    let mut s = selection();
    s.destination = FieldSelection::Asset("shinjuku".into());
    let mut runner = compile_e233(
        &profile,
        &AssetRegistry::scan(&train).unwrap(),
        &s,
        root.path(),
    )
    .unwrap();
    let events = runner.tick(Instant::now()).unwrap();
    let ScriptEvent::Present(frame) = &events[0] else {
        panic!("expected frame")
    };
    assert_eq!(frame.pixel(48, 0), Some([3, 6, 9]));
    assert_eq!(frame.pixel(48, 16), Some([0, 0, 0]));
}

#[test]
fn service_single_uses_english_left_asset_when_japanese_is_missing() {
    let root = tempfile::tempdir().unwrap();
    let train = root.path().join("train");
    fs::create_dir_all(train.join("assets/service/48x32/en")).unwrap();
    image::RgbImage::from_pixel(48, 32, image::Rgb([7, 7, 1]))
        .save(train.join("assets/service/48x32/en/local.png"))
        .unwrap();
    let profile = Profile::from_toml(
        r#"
[profile]
id='e233'
name='E233'
[e233]
[e233.assets.service]
label='種別'
[e233.assets.service.directories]
full=['assets/service/128x32']
left-ja=['assets/service/48x32/ja']
left-en=['assets/service/48x32/en']
"#,
    )
    .unwrap();
    let mut s = selection();
    s.service = FieldSelection::Asset("local".into());
    let mut runner = compile_e233(
        &profile,
        &AssetRegistry::scan(&train).unwrap(),
        &s,
        root.path(),
    )
    .unwrap();
    let events = runner.tick(Instant::now()).unwrap();
    let ScriptEvent::Present(frame) = &events[0] else {
        panic!("expected frame")
    };
    assert_eq!(frame.pixel(0, 0), Some([7, 7, 1]));
    assert_eq!(frame.pixel(48, 0), Some([0, 0, 0]));
}

#[test]
fn scroll_cycle_shows_selected_normal_pages_after_it_finishes() {
    let root = tempfile::tempdir().unwrap();
    let train = root.path().join("train");
    let image = |path: &str, width, height, color| {
        let path = train.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::RgbImage::from_pixel(width, height, image::Rgb(color))
            .save(path)
            .unwrap();
    };
    image("assets/service/48x32/ja/local.png", 48, 32, [1, 0, 0]);
    image("assets/service/48x32/en/local.png", 48, 32, [2, 0, 0]);
    image("assets/destination/80x16/ja/dest.png", 80, 16, [3, 0, 0]);
    image("assets/destination/80x16/en/dest.png", 80, 16, [4, 0, 0]);
    image("assets/destination/80x32/dest.png", 80, 32, [10, 0, 0]);
    image("assets/next-stop/80x16/ja/next.png", 80, 16, [5, 0, 0]);
    image("assets/next-stop/80x16/en/next.png", 80, 16, [6, 0, 0]);
    image("assets/route/80x16/route.png", 80, 16, [7, 0, 0]);
    image("assets/route/80x32/route.png", 80, 32, [11, 0, 0]);
    image("assets/through-route/80x16/through.png", 80, 16, [8, 0, 0]);
    image("assets/through-route/80x32/through.png", 80, 32, [12, 0, 0]);
    image("assets/service-change/80x32/change.png", 80, 32, [9, 0, 0]);
    let font_dir = root.path().join("fonts/shinonome-mincho-16");
    fs::create_dir_all(&font_dir).unwrap();
    let font = "STARTFONT 2.1\nFONTBOUNDINGBOX 1 1 0 0\nSTARTCHAR A\nENCODING 65\nBBX 1 1 0 0\nBITMAP\n1\nENDCHAR\nENDFONT\n";
    fs::write(font_dir.join("shnmk16.bdf"), font).unwrap();
    fs::write(font_dir.join("shnm8x16a.bdf"), font).unwrap();
    let mut profile =
        Profile::from_toml(include_str!("../data/trains/e233-9000/profile.toml")).unwrap();
    profile.scroll_defaults.as_mut().unwrap().start_padding = 80;
    let mut s = selection();
    s.service = FieldSelection::Asset("local".into());
    s.destination = FieldSelection::Asset("dest".into());
    s.next_stop = FieldSelection::Asset("next".into());
    s.route = FieldSelection::Asset("route".into());
    s.through_route = FieldSelection::Asset("through".into());
    s.service_change = FieldSelection::Asset("change".into());
    s.scroll_text = "A".into();
    s.scroll_speed = 1_000.0;
    let mut runner = compile_e233(
        &profile,
        &AssetRegistry::scan(&train).unwrap(),
        &s,
        root.path(),
    )
    .unwrap();
    let start = Instant::now();
    runner.tick(start).unwrap();
    let entering = runner.tick(start + Duration::from_millis(1)).unwrap();
    let destination = runner.tick(start + Duration::from_secs(3)).unwrap();
    let through = runner.tick(start + Duration::from_secs(6)).unwrap();
    let change = runner.tick(start + Duration::from_secs(9)).unwrap();
    let route = runner.tick(start + Duration::from_secs(12)).unwrap();
    let next_ja = runner.tick(start + Duration::from_secs(15)).unwrap();
    let next_en = runner.tick(start + Duration::from_secs(18)).unwrap();
    let pixel = |events: &[ScriptEvent], x, y| {
        events.iter().rev().find_map(|event| match event {
            ScriptEvent::Present(frame) => frame.pixel(x, y),
            _ => None,
        })
    };
    assert_eq!(pixel(&entering, 127, 16), Some([255, 208, 96]));
    assert_eq!(pixel(&destination, 48, 0), Some([10, 0, 0]));
    assert_eq!(pixel(&through, 48, 0), Some([12, 0, 0]));
    assert_eq!(pixel(&change, 48, 0), Some([9, 0, 0]));
    assert_eq!(pixel(&route, 48, 0), Some([11, 0, 0]));
    assert_eq!(pixel(&next_ja, 48, 16), Some([5, 0, 0]));
    assert_eq!(pixel(&next_en, 48, 16), Some([6, 0, 0]));
}
