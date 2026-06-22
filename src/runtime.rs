use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    Matrix,
    Simulator,
    Null,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuntimeConfig {
    #[serde(default = "default_backend")]
    pub backend: BackendKind,
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    #[serde(default)]
    pub simulator_path: String,
    #[serde(default = "default_brightness")]
    pub brightness: u8,
    #[serde(default)]
    pub matrix: MatrixSettings,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatrixSettings {
    #[serde(default = "default_rows")]
    pub rows: usize,
    #[serde(default = "default_cols")]
    pub cols: usize,
    #[serde(default = "default_chain")]
    pub chain_length: usize,
    #[serde(default = "default_parallel")]
    pub parallel: usize,
    #[serde(default = "default_rp1")]
    pub rp1_backend: String,
    #[serde(default)]
    pub gpio_slowdown: u32,
}
impl Default for MatrixSettings {
    fn default() -> Self {
        Self {
            rows: default_rows(),
            cols: default_cols(),
            chain_length: default_chain(),
            parallel: default_parallel(),
            rp1_backend: default_rp1(),
            gpio_slowdown: 0,
        }
    }
}
fn default_backend() -> BackendKind {
    BackendKind::Null
}
fn default_bind() -> String {
    "0.0.0.0:3000".into()
}
fn default_data_dir() -> String {
    "data".into()
}
fn default_brightness() -> u8 {
    40
}
fn default_rows() -> usize {
    32
}
fn default_cols() -> usize {
    64
}
fn default_chain() -> usize {
    2
}
fn default_parallel() -> usize {
    1
}
fn default_rp1() -> String {
    "rio".into()
}
impl MatrixSettings {
    pub fn canvas_size(&self) -> (usize, usize) {
        (self.cols * self.chain_length, self.rows * self.parallel)
    }
}
impl RuntimeConfig {
    pub fn from_toml(input: &str) -> Result<Self> {
        let value: Self = toml::from_str(input)?;
        if value.brightness > 100 {
            bail!("brightness must be 0..100");
        }
        if value.matrix.rows == 0
            || value.matrix.cols == 0
            || value.matrix.chain_length == 0
            || value.matrix.parallel == 0
        {
            bail!("matrix dimensions must be positive");
        }
        if !matches!(value.matrix.rp1_backend.as_str(), "rio" | "pio") {
            bail!("matrix.rp1_backend must be rio or pio");
        }
        Ok(value)
    }
}
