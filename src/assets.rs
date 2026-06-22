use anyhow::Result;
use image::DynamicImage;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default)]
pub struct AssetRegistry {
    by_directory: BTreeMap<String, BTreeMap<String, PathBuf>>,
}
impl AssetRegistry {
    pub fn scan(root: &Path) -> Result<Self> {
        let mut registry = Self::default();
        let assets = root.join("assets");
        if !assets.exists() {
            return Ok(registry);
        }
        for entry in walk(&assets)? {
            let path = entry?;
            if path.extension().and_then(|x| x.to_str()) != Some("png") {
                continue;
            }
            let stem = match path.file_stem().and_then(|x| x.to_str()).map(sanitize) {
                Some(Some(v)) => v,
                _ => continue,
            };
            let parent = match path
                .parent()
                .and_then(|p| p.strip_prefix(root).ok())
                .and_then(|p| p.to_str())
            {
                Some(v) => v.replace('\\', "/"),
                None => continue,
            };
            registry
                .by_directory
                .entry(parent)
                .or_default()
                .insert(stem, path);
        }
        Ok(registry)
    }
    pub fn resolve(&self, directory: &str, id: &str) -> Option<&PathBuf> {
        self.by_directory.get(directory)?.get(id)
    }
    pub fn list(&self, directory: &str) -> Vec<String> {
        self.by_directory
            .get(directory)
            .map(|x| x.keys().cloned().collect())
            .unwrap_or_default()
    }
    pub fn load_rgb(&self, directory: &str, id: &str) -> Result<(usize, usize, Vec<u8>)> {
        let path = self
            .resolve(directory, id)
            .ok_or_else(|| anyhow::anyhow!("unknown asset \"{id}\""))?;
        let image: DynamicImage = image::open(path)?;
        let image = image.to_rgb8();
        Ok((
            image.width() as usize,
            image.height() as usize,
            image.into_raw(),
        ))
    }
    pub fn validate_size(
        &self,
        directory: &str,
        id: &str,
        width: usize,
        height: usize,
    ) -> Result<()> {
        let (actual_width, actual_height, _) = self.load_rgb(directory, id)?;
        anyhow::ensure!(
            (actual_width, actual_height) == (width, height),
            "asset {directory}/{id}: expected {width}x{height}, got {actual_width}x{actual_height}"
        );
        Ok(())
    }
}
fn sanitize(value: &str) -> Option<String> {
    let value: String = value
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .to_owned();
    (!value.is_empty() && !value.starts_with('.')).then_some(value)
}
fn walk(path: &Path) -> Result<Vec<Result<PathBuf, std::io::Error>>> {
    fn collect(dir: &Path, out: &mut Vec<Result<PathBuf, std::io::Error>>) {
        match fs::read_dir(dir) {
            Ok(entries) => {
                for e in entries {
                    match e {
                        Ok(e) if e.path().is_dir() => collect(&e.path(), out),
                        Ok(e) => out.push(Ok(e.path())),
                        Err(e) => out.push(Err(e)),
                    }
                }
            }
            Err(e) => out.push(Err(e)),
        }
    }
    let mut out = Vec::new();
    collect(path, &mut out);
    Ok(out)
}
