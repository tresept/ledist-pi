#[cfg(feature = "hardware")]
use crate::MatrixSettings;
use crate::RgbFrame;
use anyhow::Result;
use image::{ImageBuffer, Rgb};
use std::path::PathBuf;

pub trait DisplayBackend: Send {
    fn present(&mut self, frame: &RgbFrame) -> Result<()>;
    fn set_brightness(&mut self, brightness: u8) -> Result<()>;
    fn blank(&mut self) -> Result<()>;
}

#[derive(Default)]
pub struct NullBackend {
    last: Option<RgbFrame>,
    brightness: u8,
}
impl NullBackend {
    pub fn last_frame(&self) -> Option<&RgbFrame> {
        self.last.as_ref()
    }
}
impl DisplayBackend for NullBackend {
    fn present(&mut self, frame: &RgbFrame) -> Result<()> {
        self.last = Some(frame.clone());
        Ok(())
    }
    fn set_brightness(&mut self, brightness: u8) -> Result<()> {
        self.brightness = brightness.min(100);
        Ok(())
    }
    fn blank(&mut self) -> Result<()> {
        if let Some(frame) = &mut self.last {
            *frame = RgbFrame::black(frame.width(), frame.height());
        }
        Ok(())
    }
}

pub struct SimulatorBackend {
    path: PathBuf,
    last: Option<RgbFrame>,
    brightness: u8,
}
impl SimulatorBackend {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            last: None,
            brightness: 100,
        }
    }
}
impl DisplayBackend for SimulatorBackend {
    fn present(&mut self, frame: &RgbFrame) -> Result<()> {
        let image = ImageBuffer::<Rgb<u8>, _>::from_raw(
            frame.width() as u32,
            frame.height() as u32,
            frame.as_rgb().to_vec(),
        )
        .expect("frame dimensions are valid");
        image.save(&self.path)?;
        self.last = Some(frame.clone());
        Ok(())
    }
    fn set_brightness(&mut self, brightness: u8) -> Result<()> {
        self.brightness = brightness.min(100);
        Ok(())
    }
    fn blank(&mut self) -> Result<()> {
        if let Some(frame) = &self.last {
            self.present(&RgbFrame::black(frame.width(), frame.height()))?;
        }
        Ok(())
    }
}

#[cfg(feature = "hardware")]
pub struct MatrixBackend {
    matrix: rust_hub75_matrix::Matrix,
    width: usize,
    height: usize,
}
#[cfg(feature = "hardware")]
impl MatrixBackend {
    pub fn new(settings: &MatrixSettings, brightness: u8) -> Result<Self> {
        use rust_hub75_matrix::{Matrix, MatrixConfig, Rp1Backend};
        let matrix = Matrix::new(MatrixConfig {
            rows: settings.rows as u32,
            cols: settings.cols as u32,
            chain_length: settings.chain_length as u32,
            parallel: settings.parallel as u32,
            brightness,
            gpio_slowdown: settings.gpio_slowdown,
            rp1_backend: if settings.rp1_backend == "pio" {
                Rp1Backend::Pio
            } else {
                Rp1Backend::Rio
            },
            ..Default::default()
        })?;
        let (width, height) = matrix.dimensions();
        Ok(Self {
            matrix,
            width,
            height,
        })
    }
}
#[cfg(feature = "hardware")]
impl DisplayBackend for MatrixBackend {
    fn present(&mut self, frame: &RgbFrame) -> Result<()> {
        anyhow::ensure!(
            frame.width() == self.width && frame.height() == self.height,
            "frame dimensions do not match HUB75 canvas"
        );
        self.matrix.present_rgb(frame.as_rgb())?;
        Ok(())
    }
    fn set_brightness(&mut self, brightness: u8) -> Result<()> {
        self.matrix.set_brightness(brightness)?;
        Ok(())
    }
    fn blank(&mut self) -> Result<()> {
        self.matrix.clear()?;
        Ok(())
    }
}
