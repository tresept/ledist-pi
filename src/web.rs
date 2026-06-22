use crate::{Profile, parse_program};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub struct AppState {
    profiles: BTreeMap<String, Profile>,
    current: Mutex<Option<DisplayState>>,
    data_dir: PathBuf,
}
#[derive(Clone, Serialize)]
pub struct DisplayState {
    profile_id: String,
    brightness: u8,
    program: String,
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
        }
    }
    pub fn with_data_dir(mut self, data_dir: impl Into<PathBuf>) -> Self {
        self.data_dir = data_dir.into();
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
    values: serde_json::Value,
    program: String,
}

pub fn web_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/app.js", get(script))
        .route("/api/profiles", get(list_profiles))
        .route("/api/profiles/{id}", get(profile))
        .route("/api/profiles/{id}/templates/{template}", get(template))
        .route("/api/display/apply", post(apply))
        .route("/api/display/blank", post(blank))
        .route("/api/display/state", get(display_state))
        .with_state(state)
}
async fn index() -> Html<&'static str> {
    Html(include_str!("../web/index.html"))
}
async fn script() -> (
    [(axum::http::header::HeaderName, &'static str); 1],
    &'static str,
) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
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
async fn template(
    Path((id, template)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<String, StatusCode> {
    if !state.profiles.contains_key(&id) || template.contains('/') || template.contains('\0') {
        return Err(StatusCode::NOT_FOUND);
    }
    std::fs::read_to_string(
        state
            .data_dir
            .join(id)
            .join("templates")
            .join(format!("{template}.txt")),
    )
    .map_err(|_| StatusCode::NOT_FOUND)
}
async fn apply(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ApplyRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if req.brightness > 100 {
        return Err((StatusCode::BAD_REQUEST, "brightness must be 0..100".into()));
    }
    if !state.profiles.contains_key(&req.profile_id) {
        return Err((StatusCode::NOT_FOUND, "unknown profile".into()));
    }
    let _ = req.values;
    parse_program(&req.program).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let next = DisplayState {
        profile_id: req.profile_id,
        brightness: req.brightness,
        program: req.program,
    };
    *state.current.lock().unwrap() = Some(next.clone());
    Ok(Json(next))
}
async fn blank(State(state): State<Arc<AppState>>) -> StatusCode {
    *state.current.lock().unwrap() = None;
    StatusCode::NO_CONTENT
}
async fn display_state(State(state): State<Arc<AppState>>) -> Json<Option<DisplayState>> {
    Json(state.current_state())
}
