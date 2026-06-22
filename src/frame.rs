use anyhow::{Result, bail};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbFrame {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl RgbFrame {
    pub fn black(width: usize, height: usize) -> Self {
        Self::solid(width, height, [0, 0, 0])
    }

    pub fn solid(width: usize, height: usize, color: [u8; 3]) -> Self {
        let mut pixels = vec![0; width * height * 3];
        for pixel in pixels.chunks_exact_mut(3) {
            pixel.copy_from_slice(&color);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn as_rgb(&self) -> &[u8] {
        &self.pixels
    }

    pub fn pixel(&self, x: usize, y: usize) -> Option<[u8; 3]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = (y * self.width + x) * 3;
        Some([self.pixels[i], self.pixels[i + 1], self.pixels[i + 2]])
    }

    pub fn clear_region(&mut self, x: usize, y: usize, width: usize, height: usize) {
        for yy in y..y.saturating_add(height).min(self.height) {
            for xx in x..x.saturating_add(width).min(self.width) {
                let i = (yy * self.width + xx) * 3;
                self.pixels[i..i + 3].fill(0);
            }
        }
    }

    pub fn blit_rgb(
        &mut self,
        x: isize,
        y: isize,
        width: usize,
        height: usize,
        source: &[u8],
    ) -> Result<()> {
        if source.len() != width * height * 3 {
            bail!("RGB source length does not match dimensions");
        }
        for sy in 0..height {
            for sx in 0..width {
                let dx = x + sx as isize;
                let dy = y + sy as isize;
                if dx < 0 || dy < 0 || dx as usize >= self.width || dy as usize >= self.height {
                    continue;
                }
                let from = (sy * width + sx) * 3;
                let to = (dy as usize * self.width + dx as usize) * 3;
                self.pixels[to..to + 3].copy_from_slice(&source[from..from + 3]);
            }
        }
        Ok(())
    }
}
