use crate::{
    AssetRegistry, DisplayCommand, E233DisplaySelection, FieldSelection, GifRunner, PreviewFrame,
    Profile, RgbFrame, ScrollCycleItem, compile_e233, compile_pattern, new_preview_frame,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc::Sender},
};

pub struct AppState {
    profiles: BTreeMap<String, Profile>,
    current: Mutex<Option<DisplayState>>,
    data_dir: PathBuf,
    display: Option<Sender<DisplayCommand>>,
    preview: PreviewFrame,
}
#[derive(Clone, Serialize)]
pub struct DisplayState {
    pub profile_id: String,
    pub brightness: u8,
}
impl AppState {
    pub fn new(profiles: Vec<Profile>) -> Self {
        Self {
            profiles: profiles
                .into_iter()
                .map(|p| (p.profile.id.clone(), p))
                .collect(),
            current: Mutex::new(None),
            data_dir: PathBuf::from("data/trains"),
            display: None,
            preview: new_preview_frame(),
        }
    }
    pub fn with_display(mut self, display: Sender<DisplayCommand>) -> Self {
        self.display = Some(display);
        self
    }
    pub fn with_data_dir(mut self, data_dir: impl Into<PathBuf>) -> Self {
        self.data_dir = data_dir.into();
        self
    }
    pub fn with_preview(mut self, preview: PreviewFrame) -> Self {
        self.preview = preview;
        self
    }
    pub fn current_state(&self) -> Option<DisplayState> {
        self.current.lock().unwrap().clone()
    }
}
#[derive(Serialize)]
struct ProfileSummary {
    id: String,
    name: String,
}
#[derive(Deserialize)]
struct ApplyRequest {
    profile_id: String,
    brightness: u8,
    #[serde(default)]
    service: Option<String>,
    #[serde(default)]
    route: Option<String>,
    #[serde(default)]
    service_change: Option<String>,
    #[serde(default)]
    through_route: Option<String>,
    #[serde(default)]
    destination: Option<String>,
    #[serde(default)]
    next_stop: Option<String>,
    #[serde(default)]
    scroll_text: String,
    #[serde(default)]
    scroll_speed: Option<f64>,
    #[serde(default)]
    scroll_cycle: Vec<ScrollCycleItem>,
}
fn selection(value: Option<String>) -> FieldSelection {
    match value.as_deref().map(str::trim) {
        None | Some("") => FieldSelection::None,
        Some(value) => FieldSelection::Asset(value.to_owned()),
    }
}

pub fn web_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/app.js", get(script))
        .route("/api/profiles", get(list_profiles))
        .route("/api/profiles/{id}", get(profile))
        .route("/api/profiles/{id}/assets/{group}", get(group_assets))
        .route("/api/display/apply", post(apply))
        .route("/api/display/stop", post(stop))
        .route("/api/display/test", post(test_display))
        .route("/api/display/blank", post(blank))
        .route("/api/display/state", get(display_state))
        .route("/api/display/preview.png", get(preview_png))
        .with_state(state)
}
async fn preview_png(State(state): State<Arc<AppState>>) -> Result<Response, StatusCode> {
    let frame = state
        .preview
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| RgbFrame::black(128, 32));
    let image = image::RgbImage::from_raw(
        frame.width() as u32,
        frame.height() as u32,
        frame.as_rgb().to_vec(),
    )
    .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(image)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(([(axum::http::header::CONTENT_TYPE, "image/png")], bytes).into_response())
}
async fn index() -> Html<&'static str> {
    Html(include_str!("../web/index.html"))
}
async fn script() -> (
    [(axum::http::header::HeaderName, &'static str); 2],
    &'static str,
) {
    (
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        include_str!("../web/app.js"),
    )
}
async fn list_profiles(State(state): State<Arc<AppState>>) -> Json<Vec<ProfileSummary>> {
    Json(
        state
            .profiles
            .values()
            .map(|p| ProfileSummary {
                id: p.profile.id.clone(),
                name: p.profile.name.clone(),
            })
            .collect(),
    )
}
async fn profile(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Profile>, StatusCode> {
    state
        .profiles
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}
async fn group_assets(
    Path((id, group)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let profile = state.profiles.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let config = profile.e233.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    let asset_group = config.assets.get(&group).ok_or(StatusCode::NOT_FOUND)?;
    let registry = AssetRegistry::scan(&state.data_dir.join(&id))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut ids = asset_group
        .directories
        .values()
        .flatten()
        .flat_map(|directory| registry.list(directory))
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    Ok(Json(ids))
}
async fn apply(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ApplyRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if req.brightness > 100 {
        return Err((StatusCode::BAD_REQUEST, "brightness must be 0..100".into()));
    }
    let profile = state
        .profiles
        .get(&req.profile_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "unknown profile".into()))?;
    let assets = AssetRegistry::scan(&state.data_dir.join(&req.profile_id))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let runner = if profile.e233.is_some() {
        let scroll_speed = req.scroll_speed.unwrap_or_else(|| {
            profile
                .scroll_defaults
                .as_ref()
                .map(|defaults| defaults.speed_px_per_second)
                .unwrap_or(50.0)
        });
        if !(1.0..=200.0).contains(&scroll_speed) {
            return Err((
                StatusCode::BAD_REQUEST,
                "scroll_speed must be 1..200".into(),
            ));
        }
        let selection = E233DisplaySelection {
            service: selection(req.service),
            route: selection(req.route),
            service_change: selection(req.service_change),
            through_route: selection(req.through_route),
            destination: selection(req.destination),
            next_stop: selection(req.next_stop),
            scroll_text: req.scroll_text,
            scroll_speed,
            scroll_cycle: req.scroll_cycle,
            brightness: req.brightness,
        };
        compile_e233(
            profile,
            &assets,
            &selection,
            state.data_dir.parent().unwrap_or(&state.data_dir),
        )
    } else {
        compile_pattern(
            profile,
            &assets,
            &state
                .data_dir
                .join(&req.profile_id)
                .join("patterns/default.toml"),
            state.data_dir.parent().unwrap_or(&state.data_dir),
        )
    }
    .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e))?;
    let display = state.display.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "display backend is unavailable".into(),
        )
    })?;
    display
        .send(DisplayCommand::SetBrightness(req.brightness))
        .map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "display worker stopped".into(),
            )
        })?;
    display
        .send(DisplayCommand::StartScript(runner))
        .map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "display worker stopped".into(),
            )
        })?;
    let next = DisplayState {
        profile_id: req.profile_id,
        brightness: req.brightness,
    };
    *state.current.lock().unwrap() = Some(next.clone());
    Ok(Json(next))
}
async fn stop(State(state): State<Arc<AppState>>) -> Result<StatusCode, (StatusCode, String)> {
    send(&state, DisplayCommand::StopScript)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn blank(State(state): State<Arc<AppState>>) -> Result<StatusCode, (StatusCode, String)> {
    send(&state, DisplayCommand::Blank)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn test_display(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, String)> {
    let path = state
        .data_dir
        .parent()
        .unwrap_or(&state.data_dir)
        .join("test.gif");
    if !path.is_file() {
        return Err((StatusCode::NOT_FOUND, "data/test.gif がありません".into()));
    }
    let gif = GifRunner::load(&path, 128, 32)
        .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;
    send(&state, DisplayCommand::StartGif(gif))?;
    Ok(StatusCode::OK)
}
async fn display_state(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DisplayState>, StatusCode> {
    state.current_state().map(Json).ok_or(StatusCode::NOT_FOUND)
}
fn send(state: &AppState, command: DisplayCommand) -> Result<(), (StatusCode, String)> {
    state
        .display
        .as_ref()
        .ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "display backend is unavailable".into(),
            )
        })?
        .send(command)
        .map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "display worker stopped".into(),
            )
        })
}
