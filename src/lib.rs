mod assets;
mod display;
mod e233;
mod font;
mod frame;
mod patterns;
mod profile;
mod runtime;
mod script;
mod web;

pub use assets::AssetRegistry;
#[cfg(feature = "hardware")]
pub use display::MatrixBackend;
pub use display::{
    DisplayBackend, DisplayCommand, GifRunner, NullBackend, PreviewFrame, SimulatorBackend,
    new_preview_frame, spawn_display_worker, spawn_display_worker_with_preview,
};
pub use e233::{
    Content as E233Content, DisplayPlan as E233DisplayPlan,
    DisplaySelection as E233DisplaySelection, FieldSelection, Layout as E233Layout,
    Page as E233Page, PageDuration as E233PageDuration, ScrollCycleItem, compile as compile_e233,
    plan as plan_e233,
};
pub use font::BdfFont;
pub use frame::RgbFrame;
pub use patterns::{Pattern, load_and_compile as compile_pattern};
pub use profile::{E233AssetGroup, E233Config, Field, Profile, Region};
pub use runtime::{BackendKind, MatrixSettings, RuntimeConfig};
pub use script::{ScriptAction, ScriptEvent, ScriptRunner, ScrollSpec};
pub use web::{AppState, web_router};
