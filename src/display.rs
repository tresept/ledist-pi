#[cfg(feature = "hardware")]
use crate::MatrixSettings;
use crate::{RgbFrame, ScriptEvent, ScriptRunner};
use anyhow::Result;
use image::{AnimationDecoder, ImageBuffer, Rgb};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

pub type PreviewFrame = Arc<Mutex<Option<RgbFrame>>>;

pub fn new_preview_frame() -> PreviewFrame {
    Arc::new(Mutex::new(None))
}

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
    StartScript(ScriptRunner),
    StartGif(GifRunner),
    StopScript,
    Blank,
}

pub struct GifRunner {
    frames: Vec<(RgbFrame, Duration)>,
    index: usize,
    next: Instant,
}
impl GifRunner {
    pub fn load(path: &Path, width: usize, height: usize) -> Result<Self> {
        let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(
            std::fs::File::open(path)?,
        ))?;
        let frames = decoder.into_frames().collect_frames()?;
        anyhow::ensure!(!frames.is_empty(), "GIFにフレームがありません");
        let mut output = Vec::with_capacity(frames.len());
        for frame in frames {
            let delay = frame_delay(&frame.delay());
            let image = frame.into_buffer();
            anyhow::ensure!(
                (image.width() as usize, image.height() as usize) == (width, height),
                "GIFの全フレームは{width}x{height}である必要があります"
            );
            let mut rgb = Vec::with_capacity(width * height * 3);
            for pixel in image.pixels() {
                rgb.extend_from_slice(&pixel.0[..3]);
            }
            output.push((RgbFrame::from_rgb(width, height, rgb)?, delay));
        }
        Ok(Self {
            frames: output,
            index: 0,
            next: Instant::now(),
        })
    }
    fn tick(&mut self, now: Instant) -> Option<RgbFrame> {
        if now < self.next {
            return None;
        }
        let (frame, delay) = &self.frames[self.index];
        self.next = now + *delay;
        self.index = (self.index + 1) % self.frames.len();
        Some(frame.clone())
    }
}
fn frame_delay(delay: &image::Delay) -> Duration {
    let (numerator, denominator) = delay.numer_denom_ms();
    Duration::from_secs_f64((numerator as f64 / denominator.max(1) as f64) / 1000.0)
        .max(Duration::from_millis(10))
}

pub fn spawn_display_worker<F>(create: F) -> anyhow::Result<mpsc::Sender<DisplayCommand>>
where
    F: FnOnce() -> Result<Box<dyn DisplayBackend>> + Send + 'static,
{
    spawn_display_worker_with_preview(create, new_preview_frame())
}

pub fn spawn_display_worker_with_preview<F>(
    create: F,
    preview: PreviewFrame,
) -> anyhow::Result<mpsc::Sender<DisplayCommand>>
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
        let mut script = None;
        let mut gif = None;
        loop {
            match receiver.recv_timeout(Duration::from_millis(33)) {
                Ok(DisplayCommand::StartScript(next)) => {
                    eprintln!("[display] script started");
                    script = Some(next);
                    gif = None;
                }
                Ok(DisplayCommand::StartGif(next)) => {
                    eprintln!("[display] GIF playback started");
                    script = None;
                    gif = Some(next);
                }
                Ok(DisplayCommand::StopScript) => {
                    eprintln!("[display] script stopped; current frame remains visible");
                    script = None;
                    gif = None;
                }
                Ok(command) => {
                    script = None;
                    gif = None;
                    run_command(&mut *backend, command, &preview)
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if let Some(runner) = &mut script {
                match runner.tick(std::time::Instant::now()) {
                    Ok(events) => {
                        for event in events {
                            match event {
                                ScriptEvent::Present(frame) => run_command(
                                    &mut *backend,
                                    DisplayCommand::Present(frame),
                                    &preview,
                                ),
                                ScriptEvent::Brightness(value) => run_command(
                                    &mut *backend,
                                    DisplayCommand::SetBrightness(value),
                                    &preview,
                                ),
                                ScriptEvent::Blank => {
                                    run_command(&mut *backend, DisplayCommand::Blank, &preview)
                                }
                            }
                        }
                    }
                    Err(error) => eprintln!("[display] script failed: {error:#}"),
                }
                if runner.is_finished() {
                    script = None;
                }
            }
            if let Some(runner) = &mut gif
                && let Some(frame) = runner.tick(Instant::now())
            {
                run_command(&mut *backend, DisplayCommand::Present(frame), &preview);
            }
        }
    });
    ready_receiver
        .recv()
        .map_err(|_| anyhow::anyhow!("display worker stopped during startup"))?
        .map_err(anyhow::Error::msg)?;
    Ok(sender)
}

fn run_command(backend: &mut dyn DisplayBackend, command: DisplayCommand, preview: &PreviewFrame) {
    let result = match command {
        DisplayCommand::Present(frame) => {
            eprintln!(
                "[display] present request: {}x{}, {} RGB bytes",
                frame.width(),
                frame.height(),
                frame.as_rgb().len()
            );
            let result = backend.present(&frame);
            if result.is_ok() {
                *preview.lock().unwrap() = Some(frame);
            }
            result
        }
        DisplayCommand::SetBrightness(brightness) => {
            eprintln!("[display] brightness request: {brightness}");
            backend.set_brightness(brightness)
        }
        DisplayCommand::Blank => {
            eprintln!("[display] blank request");
            let result = backend.blank();
            if result.is_ok() {
                let black = preview
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(|frame| RgbFrame::black(frame.width(), frame.height()));
                if let Some(black) = black {
                    *preview.lock().unwrap() = Some(black);
                }
            }
            result
        }
        DisplayCommand::StartScript(_) => return,
        DisplayCommand::StartGif(_) => return,
        DisplayCommand::StopScript => return,
    };
    match result {
        Ok(()) => eprintln!("[display] request completed"),
        Err(error) => eprintln!("[display] request failed: {error:#}"),
    }
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
