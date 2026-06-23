use anyhow::Result;
use ledist_pi::{
    AppState, BackendKind, DisplayBackend, NullBackend, Profile, RuntimeConfig, SimulatorBackend,
    new_preview_frame, spawn_display_worker_with_preview, web_router,
};
use std::{fs, net::SocketAddr, path::Path, sync::Arc};

#[tokio::main]
async fn main() -> Result<()> {
    let runtime_path =
        std::env::var("LEDIST_RUNTIME_CONFIG").unwrap_or_else(|_| "data/runtime.toml".into());
    let config = RuntimeConfig::from_toml(&fs::read_to_string(&runtime_path)?)?;
    let root = std::env::var("LEDIST_DATA_DIR").unwrap_or(config.data_dir);
    let profiles = load_profiles(Path::new(&root).join("trains"))?;
    let address: SocketAddr = std::env::var("LEDIST_BIND")
        .unwrap_or(config.bind)
        .parse()?;
    eprintln!(
        "display backend: {:?}; boot state: blank; brightness: {}",
        config.backend, config.brightness
    );
    if matches!(config.backend, BackendKind::Matrix) && !cfg!(feature = "hardware") {
        anyhow::bail!("backend=matrix requires building with --features hardware");
    }
    let backend = config.backend.clone();
    #[cfg(feature = "hardware")]
    let matrix = config.matrix.clone();
    #[cfg(feature = "hardware")]
    let brightness = config.brightness;
    let simulator_path = if config.simulator_path.is_empty() {
        "data/simulator.png".into()
    } else {
        config.simulator_path.clone()
    };
    let preview = new_preview_frame();
    let display = spawn_display_worker_with_preview(
        move || {
            let backend: Box<dyn DisplayBackend> = match backend {
                BackendKind::Null => Box::new(NullBackend::default()),
                BackendKind::Simulator => Box::new(SimulatorBackend::new(simulator_path)),
                BackendKind::Matrix => {
                    #[cfg(feature = "hardware")]
                    {
                        Box::new(ledist_pi::MatrixBackend::new(&matrix, brightness)?)
                    }
                    #[cfg(not(feature = "hardware"))]
                    {
                        anyhow::bail!("backend=matrix requires --features hardware")
                    }
                }
            };
            Ok(backend)
        },
        preview.clone(),
    )?;
    println!("LEDist UI: http://{address}");
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(
        listener,
        web_router(Arc::new(
            AppState::new(profiles)
                .with_data_dir(Path::new(&root).join("trains"))
                .with_display(display)
                .with_preview(preview),
        )),
    )
    .await?;
    Ok(())
}
fn load_profiles(dir: impl AsRef<Path>) -> Result<Vec<Profile>> {
    let mut profiles = Vec::new();
    if !dir.as_ref().exists() {
        return Ok(profiles);
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path().join("profile.toml");
        if path.exists() {
            profiles.push(Profile::from_toml(&fs::read_to_string(path)?)?);
        }
    }
    Ok(profiles)
}
