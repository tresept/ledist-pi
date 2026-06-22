use crate::{Command, FrameOp, Program, RgbFrame};
use anyhow::{Result, bail};

pub struct FrameRunner {
    frame: RgbFrame,
}
impl FrameRunner {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            frame: RgbFrame::black(width, height),
        }
    }
    pub fn frame(&self) -> &RgbFrame {
        &self.frame
    }
    pub fn apply_first_frame<F>(&mut self, program: &Program, mut asset: F) -> Result<()>
    where
        F: FnMut(&str) -> Option<(usize, usize, usize, usize, Vec<u8>)>,
    {
        let Command::Frame(operations) = program
            .commands
            .first()
            .ok_or_else(|| anyhow::anyhow!("program has no commands"))?
        else {
            bail!("program does not start with frame");
        };
        let mut next = self.frame.clone();
        for operation in operations {
            match operation {
                FrameOp::Set(_, field) | FrameOp::Scroll(_, field) => {
                    let (x, y, w, h, pixels) =
                        asset(field).ok_or_else(|| anyhow::anyhow!("unknown field {field}"))?;
                    next.blit_rgb(x as isize, y as isize, w, h, &pixels)?;
                }
                FrameOp::Clear(_) => {}
            }
        }
        self.frame = next;
        Ok(())
    }
}
