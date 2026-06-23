use axum::{
    body::Body,
    http::{Request, StatusCode, header::CONTENT_TYPE},
};
use image::GenericImageView;
use ledist_pi::{AppState, NullBackend, Profile, spawn_display_worker, web_router};
use std::{fs, sync::Arc};
use tower::ServiceExt;

fn e233_profile() -> Profile {
    Profile::from_toml(
        r#"
[profile]
id='e233'
name='E233'
default_brightness=40
[e233]
[e233.assets.service]
label='種別'
[e233.assets.service.directories]
full=['assets/service/128x32']
left-ja=['assets/service/48x32/ja']
left-en=['assets/service/48x32/en']
[e233.assets.route]
label='路線名'
[e233.assets.route.directories]
full=['assets/route/full']
full-top=['assets/route/full-top']
right=['assets/route/80x32']
right-top=['assets/route/80x16']
[e233.assets.service_change]
label='種別変更'
[e233.assets.service_change.directories]
right=['assets/service-change/right']
[e233.assets.through_route]
label='直通先'
[e233.assets.through_route.directories]
right=['assets/through-route/80x32']
right-top=['assets/through-route/80x16']
[e233.assets.destination]
label='行先'
[e233.assets.destination.directories]
full=['assets/destination/128x32']
right=['assets/destination/80x32']
full-top=['assets/destination/128x16']
right-top-ja=['assets/destination/80x16/ja']
right-top-en=['assets/destination/80x16/en']
[e233.assets.next_stop]
label='次駅'
[e233.assets.next_stop.directories]
right-bottom-ja=['assets/next-stop/80x16/ja']
right-bottom-en=['assets/next-stop/80x16/en']
full-bottom-ja=['assets/next-stop/128x16/ja']
full-bottom-en=['assets/next-stop/128x16/en']
"#,
    )
    .unwrap()
}
#[tokio::test]
async fn profiles_endpoint_returns_registered_profile() {
    let app = web_router(Arc::new(AppState::new(vec![e233_profile()])));
    let response = app
        .oneshot(Request::get("/api/profiles").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn preview_endpoint_returns_a_128_by_32_png() {
    let app = web_router(Arc::new(AppState::new(vec![])));
    let response = app
        .oneshot(
            Request::get("/api/display/preview.png")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CONTENT_TYPE], "image/png");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let image = image::load_from_memory(&body).unwrap();
    assert_eq!(image.dimensions(), (128, 32));
}
#[tokio::test]
async fn canvas_preview_endpoint_returns_128_by_32_rgb_bytes() {
    let app = web_router(Arc::new(AppState::new(vec![])));
    let response = app
        .oneshot(
            Request::get("/api/display/preview.rgb")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/octet-stream");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body.len(), 128 * 32 * 3);
}
#[tokio::test]
async fn invalid_apply_leaves_display_state_unchanged() {
    let state = Arc::new(AppState::new(vec![e233_profile()]));
    let app = web_router(state.clone());
    let response = app
        .oneshot(
            Request::post("/api/display/apply")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"profile_id":"e233","brightness":101}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(state.current_state().is_none());
}
#[tokio::test]
async fn e233_group_assets_are_discovered_and_blank_is_accepted() {
    let data = tempfile::tempdir().unwrap();
    let train = data.path().join("e233");
    fs::create_dir_all(train.join("assets/service/128x32")).unwrap();
    image::RgbImage::new(128, 32)
        .save(train.join("assets/service/128x32/local.png"))
        .unwrap();
    let display = spawn_display_worker(|| Ok(Box::new(NullBackend::default()))).unwrap();
    let state = Arc::new(
        AppState::new(vec![e233_profile()])
            .with_data_dir(data.path())
            .with_display(display),
    );
    let app = web_router(state.clone());
    let assets = app
        .clone()
        .oneshot(
            Request::get("/api/profiles/e233/assets/service")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(assets.status(), StatusCode::OK);
    let ok = app
        .oneshot(
            Request::post("/api/display/apply")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"profile_id":"e233","brightness":20,"service":"local"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
    assert!(state.current_state().is_some());
}
#[tokio::test]
async fn test_display_requires_a_128_by_32_test_gif() {
    let data = tempfile::tempdir().unwrap();
    let display = spawn_display_worker(|| Ok(Box::new(NullBackend::default()))).unwrap();
    let state = Arc::new(
        AppState::new(vec![])
            .with_data_dir(data.path().join("trains"))
            .with_display(display),
    );
    let app = web_router(state);
    let missing = app
        .clone()
        .oneshot(
            Request::post("/api/display/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    let file = std::fs::File::create(data.path().join("test.gif")).unwrap();
    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    encoder
        .encode_frame(image::Frame::new(image::RgbaImage::new(128, 32)))
        .unwrap();
    drop(encoder);
    let ok = app
        .oneshot(
            Request::post("/api/display/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = ok.status();
    let body = axum::body::to_bytes(ok.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
}
#[tokio::test]
async fn blank_returns_service_unavailable_without_a_display_worker() {
    let app = web_router(Arc::new(AppState::new(vec![])));
    let response = app
        .oneshot(
            Request::post("/api/display/blank")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
