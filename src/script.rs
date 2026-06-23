use crate::{BdfFont, Region, RgbFrame};
use anyhow::Result;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct ScrollSpec {
    pub region: Region,
    pub text: String,
    pub font: Arc<BdfFont>,
    pub color: [u8; 3],
    pub speed_px_per_second: f64,
    pub start_padding: usize,
    pub end_padding: usize,
    pub repeat: bool,
}
#[derive(Clone)]
pub enum ScriptAction {
    Present {
        frame: RgbFrame,
        scroll: Option<ScrollSpec>,
    },
    Wait(Duration),
    WaitScrollEnd,
    Brightness(u8),
    Blank,
    WhileScroll(Arc<Vec<ScriptAction>>),
    CheckScroll,
}
pub enum ScriptEvent {
    Present(RgbFrame),
    Brightness(u8),
    Blank,
}

pub struct ScriptRunner {
    actions: Vec<ScriptAction>,
    cycle: Option<Vec<ScriptAction>>,
    index: usize,
    current: RgbFrame,
    wait_until: Option<Instant>,
    scroll: Option<(ScrollSpec, Instant)>,
    finished: bool,
    skip_until_while: bool,
}
impl ScriptRunner {
    pub fn new(
        width: usize,
        height: usize,
        actions: Vec<ScriptAction>,
        cycle: Option<Vec<ScriptAction>>,
    ) -> Self {
        Self {
            actions,
            cycle,
            index: 0,
            current: RgbFrame::black(width, height),
            wait_until: None,
            scroll: None,
            finished: false,
            skip_until_while: false,
        }
    }
    pub fn is_finished(&self) -> bool {
        self.finished
    }
    fn render_scroll(&self, now: Instant) -> Result<RgbFrame> {
        let mut frame = self.current.clone();
        let Some((scroll, started)) = &self.scroll else {
            return Ok(frame);
        };
        frame.clear_region(
            scroll.region.x,
            scroll.region.y,
            scroll.region.width,
            scroll.region.height,
        );
        let offset = (now.duration_since(*started).as_secs_f64() * scroll.speed_px_per_second)
            .floor() as isize;
        // Start inside the visible region so the first glyph is visible as soon
        // as the page is presented. `start_padding` remains available as an
        // intentional left inset, rather than an off-screen delay.
        let x = scroll.start_padding as isize - offset;
        let mut layer = RgbFrame::black(scroll.region.width, scroll.region.height);
        scroll
            .font
            .draw(&scroll.text, &mut layer, x, 0, scroll.color)?;
        frame.blit_rgb(
            scroll.region.x as isize,
            scroll.region.y as isize,
            scroll.region.width,
            scroll.region.height,
            layer.as_rgb(),
        )?;
        Ok(frame)
    }
    fn scroll_finished(&self, now: Instant) -> bool {
        let Some((scroll, started)) = &self.scroll else {
            return true;
        };
        if scroll.repeat {
            return false;
        }
        let offset = (now.duration_since(*started).as_secs_f64() * scroll.speed_px_per_second)
            .floor() as isize;
        let x = scroll.start_padding as isize - offset;
        x + scroll.font.measure(&scroll.text) as isize + (scroll.end_padding as isize) < 0
    }
    pub fn tick(&mut self, now: Instant) -> Result<Vec<ScriptEvent>> {
        let mut events = Vec::new();
        if self.finished {
            return Ok(events);
        }
        if let Some(until) = self.wait_until {
            if now < until {
                if self.scroll.is_some() {
                    events.push(ScriptEvent::Present(self.render_scroll(now)?));
                }
                return Ok(events);
            }
            self.wait_until = None;
        }
        if self.scroll_finished(now) {
            self.scroll = None;
        }
        if self.scroll.is_some() {
            events.push(ScriptEvent::Present(self.render_scroll(now)?));
        }
        loop {
            if self.index == self.actions.len() {
                if let Some(cycle) = &self.cycle {
                    self.actions = cycle.clone();
                    self.index = 0;
                } else {
                    self.finished = true;
                    break;
                }
            }
            let action = self.actions[self.index].clone();
            self.index += 1;
            if self.skip_until_while && !matches!(action, ScriptAction::WhileScroll(_)) {
                continue;
            }
            match action {
                ScriptAction::Present { frame, scroll } => {
                    self.current = frame;
                    if let Some(scroll) = scroll {
                        self.scroll = Some((scroll, now));
                    }
                    events.push(ScriptEvent::Present(self.render_scroll(now)?));
                }
                ScriptAction::Wait(duration) => {
                    self.wait_until = Some(now + duration);
                    break;
                }
                ScriptAction::WaitScrollEnd => {
                    if self.scroll.is_some() {
                        self.index -= 1;
                        break;
                    }
                }
                ScriptAction::Brightness(value) => events.push(ScriptEvent::Brightness(value)),
                ScriptAction::Blank => {
                    self.current = RgbFrame::black(self.current.width(), self.current.height());
                    self.scroll = None;
                    events.push(ScriptEvent::Blank);
                }
                ScriptAction::WhileScroll(body) => {
                    self.skip_until_while = false;
                    if self.scroll.is_some() {
                        let repeat = body.clone();
                        self.actions.splice(
                            self.index..self.index,
                            body.iter()
                                .cloned()
                                .chain(std::iter::once(ScriptAction::WhileScroll(repeat))),
                        );
                    }
                }
                ScriptAction::CheckScroll => {
                    if self.scroll.is_none() {
                        self.skip_until_while = true;
                    }
                }
            }
        }
        Ok(events)
    }
}
