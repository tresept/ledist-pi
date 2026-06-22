use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Profile {
    pub profile: ProfileMeta,
    #[serde(default)]
    pub regions: BTreeMap<String, Region>,
    #[serde(default)]
    pub fields: Vec<Field>,
    #[serde(default)]
    pub scroll_defaults: Option<ScrollDefaults>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProfileMeta {
    pub id: String,
    pub name: String,
    #[serde(default = "width_default")]
    pub canvas_width: usize,
    #[serde(default = "height_default")]
    pub canvas_height: usize,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub default_brightness: u8,
    #[serde(default)]
    pub default_template: String,
}
fn width_default() -> usize {
    128
}
fn height_default() -> usize {
    32
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Region {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Field {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub asset_dir: Option<String>,
    #[serde(default)]
    pub target_region: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub require_exact_size: bool,
    #[serde(default)]
    pub default: Option<toml::Value>,
    #[serde(default)]
    pub visible_when: Option<String>,
    #[serde(default)]
    pub options: Vec<FieldOption>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FieldOption {
    pub value: String,
    pub label: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScrollDefaults {
    pub font: String,
    pub color: String,
    pub speed_px_per_second: f64,
    pub start_padding: usize,
    pub end_padding: usize,
    #[serde(default)]
    pub repeat: bool,
}

impl Profile {
    pub fn from_toml(text: &str) -> Result<Self> {
        let profile: Self = toml::from_str(text).context("invalid profile TOML")?;
        profile.validate()?;
        Ok(profile)
    }
    pub fn validate(&self) -> Result<()> {
        if self.profile.id.trim().is_empty() {
            bail!("profile id must not be empty");
        }
        for (name, region) in &self.regions {
            if region.width == 0
                || region.height == 0
                || region.x.saturating_add(region.width) > self.profile.canvas_width
                || region.y.saturating_add(region.height) > self.profile.canvas_height
            {
                bail!("region {name} is outside the canvas");
            }
        }
        for field in &self.fields {
            if let Some(region) = &field.target_region
                && !self.regions.contains_key(region)
            {
                bail!("field {} references unknown region {region}", field.id);
            }
        }
        Ok(())
    }
}
