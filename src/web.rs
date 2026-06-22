use crate::{
    AssetRegistry, Command, DisplayCommand, FrameOp, Profile, Program, RgbFrame, parse_program,
};
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
    sync::{Arc, Mutex, mpsc::Sender},
};

pub struct AppState {
    profiles: BTreeMap<String, Profile>,
    assets: BTreeMap<String, AssetRegistry>,
    current: Mutex<Option<DisplayState>>,
    data_dir: PathBuf,
    display: Option<Sender<DisplayCommand>>,
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
            assets: BTreeMap::new(),
            current: Mutex::new(None),
            data_dir: PathBuf::from("data/trains"),
            display: None,
        }
    }
    pub fn with_display(mut self, display: Sender<DisplayCommand>) -> Self {
        self.display = Some(display);
        self
    }
    pub fn with_data_dir(mut self, data_dir: impl Into<PathBuf>) -> Self {
        self.data_dir = data_dir.into();
        self.assets = self
            .profiles
            .keys()
            .filter_map(|id| {
                AssetRegistry::scan(&self.data_dir.join(id))
                    .ok()
                    .map(|assets| (id.clone(), assets))
            })
            .collect();
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
        .route("/api/profiles/{id}/assets/{field}", get(field_assets))
        .route("/api/profiles/{id}/templates/{template}", get(template))
        .route("/api/display/apply", post(apply))
        .route("/api/display/test", post(test_display))
        .route("/api/display/blank", post(blank))
        .route("/api/display/state", get(display_state))
        .with_state(state)
}
async fn field_assets(
    Path((id, field)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let profile = state.profiles.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let field = profile
        .fields
        .iter()
        .find(|value| value.id == field)
        .ok_or(StatusCode::NOT_FOUND)?;
    let directory = field.asset_dir.as_deref().ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(
        state
            .assets
            .get(&id)
            .map(|assets| assets.list(directory))
            .unwrap_or_default(),
    ))
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
    let profile = state.profiles.get(&req.profile_id).expect("checked above");
    let values = req.values.as_object().ok_or_else(|| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "values must be an object".into(),
        )
    })?;
    validate_values(profile, state.assets.get(&req.profile_id), values)
        .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e))?;
    let program = parse_program(&req.program)
        .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;
    validate_program(profile, &program).map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e))?;
    let next = DisplayState {
        profile_id: req.profile_id,
        brightness: req.brightness,
        program: req.program,
    };
    *state.current.lock().unwrap() = Some(next.clone());
    Ok(Json(next))
}

async fn test_display(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, String)> {
    let path = state
        .data_dir
        .parent()
        .unwrap_or(&state.data_dir)
        .join("test.png");
    if !path.is_file() {
        return Err((StatusCode::NOT_FOUND, "data/test.png がありません".into()));
    }
    let (width, height) = image::image_dimensions(&path)
        .map_err(|error| (StatusCode::UNPROCESSABLE_ENTITY, error.to_string()))?;
    if (width, height) != (128, 32) {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("data/test.png は128x32である必要があります（現在: {width}x{height}）"),
        ));
    }
    let pixels = image::open(&path)
        .map_err(|error| (StatusCode::UNPROCESSABLE_ENTITY, error.to_string()))?
        .to_rgb8()
        .into_raw();
    let mut frame = RgbFrame::black(128, 32);
    frame
        .blit_rgb(0, 0, 128, 32, &pixels)
        .map_err(|error| (StatusCode::UNPROCESSABLE_ENTITY, error.to_string()))?;
    state
        .display
        .as_ref()
        .ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "display backend is unavailable".into(),
            )
        })?
        .send(DisplayCommand::Present(frame))
        .map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "display worker stopped".into(),
            )
        })?;
    Ok(StatusCode::OK)
}

fn validate_values(
    profile: &Profile,
    assets: Option<&AssetRegistry>,
    values: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    for field in &profile.fields {
        let value = values.get(&field.id);
        if field.required
            && value
                .and_then(serde_json::Value::as_str)
                .is_none_or(str::is_empty)
        {
            return Err(format!("required field {} is missing", field.id));
        }
        let Some(value) = value else { continue };
        if let (Some(min), Some(number)) = (field.min, value.as_f64())
            && number < min
        {
            return Err(format!("field {} is below minimum", field.id));
        }
        if let (Some(max), Some(number)) = (field.max, value.as_f64())
            && number > max
        {
            return Err(format!("field {} is above maximum", field.id));
        }
        if field.kind == "select"
            && !field
                .options
                .iter()
                .any(|option| value.as_str() == Some(&option.value))
        {
            return Err(format!("invalid option for {}", field.id));
        }
        if field.kind == "asset" {
            let Some(id) = value.as_str().filter(|value| !value.is_empty()) else {
                continue;
            };
            let directory = field
                .asset_dir
                .as_deref()
                .ok_or_else(|| format!("asset field {} has no asset_dir", field.id))?;
            let registry = assets
                .ok_or_else(|| format!("assets for {} are unavailable", profile.profile.id))?;
            let path = registry
                .resolve(directory, id)
                .ok_or_else(|| format!("unknown asset {id} for {}", field.id))?;
            if field.require_exact_size {
                let region_id = field
                    .target_region
                    .as_deref()
                    .ok_or_else(|| format!("asset field {} has no target region", field.id))?;
                let region = profile
                    .regions
                    .get(region_id)
                    .ok_or_else(|| format!("unknown region {region_id}"))?;
                registry
                    .validate_size(directory, id, region.width, region.height)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
            }
        }
    }
    Ok(())
}
fn validate_program(profile: &Profile, program: &Program) -> Result<(), String> {
    fn commands(profile: &Profile, entries: &[Command]) -> Result<(), String> {
        for command in entries {
            match command {
                Command::Frame(operations) => {
                    for op in operations {
                        match op {
                            FrameOp::Set(region, field) | FrameOp::Scroll(region, field) => {
                                check_region(profile, region)?;
                                check_field(profile, field)?;
                            }
                            FrameOp::Clear(region) => check_region(profile, region)?,
                        }
                    }
                }
                Command::Scroll(region, field) => {
                    check_region(profile, region)?;
                    check_field(profile, field)?;
                }
                Command::WaitField(field) => check_field(profile, field)?,
                Command::Loop(_, body) => commands(profile, body)?,
                _ => {}
            }
        }
        Ok(())
    }
    fn check_region(profile: &Profile, id: &str) -> Result<(), String> {
        if profile.regions.contains_key(id) {
            Ok(())
        } else {
            Err(format!("unknown region {id}"))
        }
    }
    fn check_field(profile: &Profile, id: &str) -> Result<(), String> {
        if profile.fields.iter().any(|field| field.id == id) {
            Ok(())
        } else {
            Err(format!("unknown field {id}"))
        }
    }
    commands(profile, &program.commands)
}
async fn blank(State(state): State<Arc<AppState>>) -> StatusCode {
    *state.current.lock().unwrap() = None;
    StatusCode::NO_CONTENT
}
async fn display_state(State(state): State<Arc<AppState>>) -> Json<Option<DisplayState>> {
    Json(state.current_state())
}
