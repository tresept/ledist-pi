#[cfg(feature = "hardware")]
use crate::MatrixSettings;
use crate::RgbFrame;
use anyhow::Result;
use image::{ImageBuffer, Rgb};
use std::{path::PathBuf, sync::mpsc, thread};

/// A display backend is used only by its owning display loop.
///
/// The HUB75 FFI handle is deliberately not `Send`, so commands must be
/// delivered to that loop rather than moving a backend between threads.
pub trait DisplayBackend {
    fn present(&mut self, frame: &RgbFrame) -> Result<()>;
    fn set_brightness(&mut self, brightness: u8) -> Result<()>;
    fn blank(&mut self) -> Result<()>;
}

pub enum DisplayCommand {
    Present(RgbFrame),
    SetBrightness(u8),
    Blank,
}

pub fn spawn_display_worker<F>(create: F) -> anyhow::Result<mpsc::Sender<DisplayCommand>>
where
    F: FnOnce() -> Result<Box<dyn DisplayBackend>> + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut backend = match create() {
            Ok(backend) => {
                eprintln!("[display] backend initialized");
                let _ = ready_sender.send(Ok(()));
                backend
            }
            Err(error) => {
                let _ = ready_sender.send(Err(error.to_string()));
                return;
            }
        };
        while let Ok(command) = receiver.recv() {
            let result = match command {
                DisplayCommand::Present(frame) => {
                    eprintln!(
                        "[display] present request: {}x{}, {} RGB bytes",
                        frame.width(),
                        frame.height(),
                        frame.as_rgb().len()
                    );
                    backend.present(&frame)
                }
                DisplayCommand::SetBrightness(brightness) => {
                    eprintln!("[display] brightness request: {brightness}");
                    backend.set_brightness(brightness)
                }
                DisplayCommand::Blank => {
                    eprintln!("[display] blank request");
                    backend.blank()
                }
            };
            match result {
                Ok(()) => eprintln!("[display] request completed"),
                Err(error) => eprintln!("[display] request failed: {error:#}"),
            }
        }
    });
    ready_receiver
        .recv()
        .map_err(|_| anyhow::anyhow!("display worker stopped during startup"))?
        .map_err(anyhow::Error::msg)?;
    Ok(sender)
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
        eprintln!(
            "[matrix] initialized: logical canvas {width}x{height}, panel {}x{}, chain={}, parallel={}, brightness={brightness}, rp1={}",
            settings.cols,
            settings.rows,
            settings.chain_length,
            settings.parallel,
            settings.rp1_backend
        );
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
        eprintln!("[matrix] calling Matrix::present_rgb()");
        self.matrix.present_rgb(frame.as_rgb())?;
        eprintln!("[matrix] Matrix::present_rgb() succeeded");
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
