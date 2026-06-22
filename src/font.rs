use crate::RgbFrame;
use anyhow::{Result, bail};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct BdfFont {
    glyphs: BTreeMap<char, Glyph>,
}
#[derive(Clone, Debug)]
struct Glyph {
    width: usize,
    height: usize,
    rows: Vec<u32>,
}
impl BdfFont {
    pub fn parse_bdf(source: &str) -> Result<Self> {
        let mut glyphs = BTreeMap::new();
        let mut encoding = None;
        let mut width = 0usize;
        let mut height = 0usize;
        let mut rows = Vec::new();
        let mut bitmap = false;
        for line in source.lines() {
            let words: Vec<_> = line.split_whitespace().collect();
            match words.as_slice() {
                ["STARTCHAR", ..] => {
                    encoding = None;
                    width = 0;
                    height = 0;
                    rows.clear();
                    bitmap = false;
                }
                ["ENCODING", code] => encoding = code.parse::<u32>().ok().and_then(char::from_u32),
                ["BBX", w, h, ..] => {
                    width = w.parse()?;
                    height = h.parse()?;
                }
                ["BITMAP"] => bitmap = true,
                ["ENDCHAR"] => {
                    if let Some(ch) = encoding {
                        if rows.len() != height {
                            bail!("glyph {ch} has invalid bitmap height");
                        }
                        glyphs.insert(
                            ch,
                            Glyph {
                                width,
                                height,
                                rows: rows.clone(),
                            },
                        );
                    }
                    bitmap = false;
                }
                _ if bitmap => rows.push(u32::from_str_radix(line.trim(), 16)?),
                _ => {}
            }
        }
        Ok(Self { glyphs })
    }
    pub fn measure(&self, text: &str) -> usize {
        text.chars()
            .filter_map(|ch| self.glyphs.get(&ch))
            .map(|g| g.width)
            .sum()
    }
    pub fn measure_checked(&self, text: &str) -> Result<usize> {
        let mut total = 0;
        for ch in text.chars() {
            total += self
                .glyphs
                .get(&ch)
                .ok_or_else(|| anyhow::anyhow!("font has no glyph for {ch}"))?
                .width;
        }
        Ok(total)
    }
    pub fn height(&self) -> usize {
        self.glyphs
            .values()
            .map(|glyph| glyph.height)
            .max()
            .unwrap_or(0)
    }
    pub fn draw(
        &self,
        text: &str,
        frame: &mut RgbFrame,
        x: isize,
        y: isize,
        color: [u8; 3],
    ) -> Result<()> {
        self.measure_checked(text)?;
        let mut pen = x;
        for character in text.chars() {
            let glyph = &self.glyphs[&character];
            for (gy, row) in glyph.rows.iter().enumerate() {
                for gx in 0..glyph.width {
                    if row & (1 << (glyph.width.saturating_sub(1) - gx)) == 0 {
                        continue;
                    }
                    let dx = pen + gx as isize;
                    let dy = y + gy as isize;
                    if dx >= 0
                        && dy >= 0
                        && (dx as usize) < frame.width()
                        && (dy as usize) < frame.height()
                    {
                        frame.blit_rgb(dx, dy, 1, 1, &color)?;
                    }
                }
            }
            pen += glyph.width as isize;
        }
        Ok(())
    }
}
