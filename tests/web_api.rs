use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use ledist_pi::{AppState, NullBackend, Profile, spawn_display_worker, web_router};
use std::{fs, sync::Arc};
use tower::ServiceExt;

#[tokio::test]
async fn profiles_endpoint_returns_registered_profile() {
    let profile =
        Profile::from_toml("[profile]\nid='e233'\nname='E233'\ncanvas_width=128\ncanvas_height=32")
            .unwrap();
    let app = web_router(Arc::new(AppState::new(vec![profile])));
    let response = app
        .oneshot(Request::get("/api/profiles").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn invalid_apply_leaves_display_state_unchanged() {
    let profile =
        Profile::from_toml("[profile]\nid='e233'\nname='E233'\ncanvas_width=128\ncanvas_height=32")
            .unwrap();
    let state = Arc::new(AppState::new(vec![profile]));
    let app = web_router(state.clone());
    let response = app
        .oneshot(
            Request::post("/api/display/apply")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"profile_id":"e233","brightness":101,"values":{},"program":"blank"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(state.current_state().is_none());
}

#[tokio::test]
async fn apply_validates_registered_assets_and_exposes_field_assets() {
    let data = tempfile::tempdir().unwrap();
    let train = data.path().join("e233");
    fs::create_dir_all(train.join("assets/service")).unwrap();
    image::RgbImage::new(2, 1)
        .save(train.join("assets/service/local.png"))
        .unwrap();
    let profile = Profile::from_toml(
        r#"
[profile]
id='e233'
name='E233'
canvas_width=2
canvas_height=1
[regions.service]
x=0
y=0
width=2
height=1
[[fields]]
id='service'
label='Service'
type='asset'
asset_dir='assets/service'
target_region='service'
required=true
require_exact_size=true
"#,
    )
    .unwrap();
    let display = spawn_display_worker(|| Ok(Box::new(NullBackend::default()))).unwrap();
    let state = Arc::new(
        AppState::new(vec![profile])
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
    let ok = app.oneshot(Request::post("/api/display/apply").header("content-type", "application/json").body(Body::from(r#"{"profile_id":"e233","brightness":20,"values":{"service":"local"},"program":"frame\n set service service\nend"}"#)).unwrap()).await.unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
    assert!(state.current_state().is_some());
}

#[tokio::test]
async fn field_assets_include_pngs_added_after_server_start() {
    let data = tempfile::tempdir().unwrap();
    let train = data.path().join("e233");
    fs::create_dir_all(train.join("assets/service")).unwrap();
    let profile = Profile::from_toml(
        r#"
[profile]
id='e233'
name='E233'
[[fields]]
id='service'
label='Service'
type='asset'
asset_dir='assets/service'
"#,
    )
    .unwrap();
    let state = Arc::new(AppState::new(vec![profile]).with_data_dir(data.path()));
    image::RgbImage::new(1, 1)
        .save(train.join("assets/service/new.png"))
        .unwrap();
    let app = web_router(state);
    let response = app
        .oneshot(
            Request::get("/api/profiles/e233/assets/service")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(bytes, "[\"new\"]");
}

#[tokio::test]
async fn test_display_endpoint_requires_a_128_by_32_test_png() {
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
    image::RgbImage::new(128, 32)
        .save(data.path().join("test.png"))
        .unwrap();
    let ok = app
        .oneshot(
            Request::post("/api/display/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
}

#[tokio::test]
async fn blank_returns_service_unavailable_without_a_display_worker() {
    let state = Arc::new(AppState::new(vec![]));
    let app = web_router(state);
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
