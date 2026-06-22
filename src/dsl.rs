use anyhow::{Result, bail};

#[derive(Clone, Debug)]
pub struct Program {
    pub commands: Vec<Command>,
}
#[derive(Clone, Debug)]
pub enum Command {
    Frame(Vec<FrameOp>),
    WaitSeconds(f64),
    WaitField(String),
    WaitScrollEnd,
    Loop(Option<usize>, Vec<Command>),
    Scroll(String, String),
    Brightness(u8),
    Blank,
}
#[derive(Clone, Debug)]
pub enum FrameOp {
    Set(String, String),
    Clear(String),
    Scroll(String, String),
}

pub fn parse_program(source: &str) -> Result<Program> {
    let lines: Vec<_> = source.lines().enumerate().collect();
    let (commands, _) = block(&lines, 0, false)?;
    Ok(Program { commands })
}
fn block(lines: &[(usize, &str)], mut i: usize, until_end: bool) -> Result<(Vec<Command>, usize)> {
    let mut commands = Vec::new();
    while i < lines.len() {
        let (line_no, raw) = lines[i];
        let words: Vec<_> = raw.split_whitespace().collect();
        i += 1;
        if words.is_empty() {
            continue;
        }
        if words[0] == "end" {
            if until_end {
                return Ok((commands, i));
            }
            bail!("{}行目: 対応しないendです", line_no + 1);
        }
        match words.as_slice() {
            ["frame"] => {
                let mut ops = Vec::new();
                loop {
                    if i >= lines.len() {
                        bail!("{}行目: frameに対応するendがありません", line_no + 1);
                    }
                    let (n, text) = lines[i];
                    i += 1;
                    let w: Vec<_> = text.split_whitespace().collect();
                    if w.is_empty() {
                        continue;
                    }
                    if w[0] == "end" {
                        break;
                    }
                    match w.as_slice() {
                        ["set", region, field] => {
                            ops.push(FrameOp::Set((*region).into(), (*field).into()))
                        }
                        ["clear", region] => ops.push(FrameOp::Clear((*region).into())),
                        ["scroll", region, field] => {
                            ops.push(FrameOp::Scroll((*region).into(), (*field).into()))
                        }
                        _ => bail!("{}行目: frame内の命令が不正です", n + 1),
                    }
                }
                commands.push(Command::Frame(ops));
            }
            ["loop"] | ["loop", _] => {
                let count =
                    if words.len() == 2 {
                        Some(words[1].parse().map_err(|_| {
                            anyhow::anyhow!("{}行目: loop回数が不正です", line_no + 1)
                        })?)
                    } else {
                        None
                    };
                let (body, next) = block(lines, i, true)?;
                i = next;
                commands.push(Command::Loop(count, body));
            }
            ["wait", "scroll_end"] => commands.push(Command::WaitScrollEnd),
            ["wait", value] if value.starts_with("${") && value.ends_with('}') => {
                commands.push(Command::WaitField(value[2..value.len() - 1].into()))
            }
            ["wait", value] => commands
                .push(Command::WaitSeconds(duration(value).ok_or_else(|| {
                    anyhow::anyhow!("{}行目: 時間指定が不正です", line_no + 1)
                })?)),
            ["scroll", region, field] => {
                commands.push(Command::Scroll((*region).into(), (*field).into()))
            }
            ["brightness", n] => {
                let brightness: u8 = n
                    .parse()
                    .map_err(|_| anyhow::anyhow!("{}行目: 輝度が不正です", line_no + 1))?;
                if brightness > 100 {
                    bail!("{}行目: 輝度は0..100です", line_no + 1);
                }
                commands.push(Command::Brightness(brightness));
            }
            ["blank"] => commands.push(Command::Blank),
            _ => bail!("{}行目: 不明な命令 \"{}\"", line_no + 1, words[0]),
        }
    }
    if until_end {
        bail!(
            "{}行目: loopに対応するendがありません",
            lines.first().map(|(n, _)| n + 1).unwrap_or(1)
        );
    }
    Ok((commands, i))
}
fn duration(value: &str) -> Option<f64> {
    value
        .strip_suffix("ms")
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| v / 1000.0)
        .or_else(|| value.strip_suffix('s').and_then(|v| v.parse().ok()))
}
