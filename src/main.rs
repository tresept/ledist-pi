use anyhow::Result;
use ledist_pi::{AppState, Profile, web_router};
use std::{fs, net::SocketAddr, path::Path, sync::Arc};

#[tokio::main]
async fn main() -> Result<()> {
    let root = std::env::var("LEDIST_DATA_DIR").unwrap_or_else(|_| "data".into());
    let profiles = load_profiles(Path::new(&root).join("trains"))?;
    let address: SocketAddr = std::env::var("LEDIST_BIND")
        .unwrap_or_else(|_| "0.0.0.0:3000".into())
        .parse()?;
    println!("LEDist UI: http://{address}");
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(
        listener,
        web_router(Arc::new(
            AppState::new(profiles).with_data_dir(Path::new(&root).join("trains")),
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
