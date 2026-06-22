mod assets;
mod display;
mod dsl;
mod font;
mod frame;
mod profile;
mod runner;
mod runtime;
mod web;

pub use assets::AssetRegistry;
#[cfg(feature = "hardware")]
pub use display::MatrixBackend;
pub use display::{
    DisplayBackend, DisplayCommand, NullBackend, SimulatorBackend, spawn_display_worker,
};
pub use dsl::{Command, FrameOp, Program, parse_program};
pub use font::BdfFont;
pub use frame::RgbFrame;
pub use profile::{Field, Profile, Region};
pub use runner::FrameRunner;
pub use runtime::{BackendKind, MatrixSettings, RuntimeConfig};
pub use web::{AppState, web_router};
