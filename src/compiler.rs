use crate::{
    AssetRegistry, BdfFont, Command, FrameOp, Profile, Program, RgbFrame, ScriptAction,
    ScriptRunner, ScrollSpec,
};
use std::{path::Path, sync::Arc, time::Duration};

pub fn compile_program(
    profile: &Profile,
    assets: &AssetRegistry,
    values: &serde_json::Map<String, serde_json::Value>,
    program: &Program,
    data_root: &Path,
) -> Result<ScriptRunner, String> {
    let font = load_font(profile, data_root)?;
    let mut frame = RgbFrame::black(profile.profile.canvas_width, profile.profile.canvas_height);
    let mut actions = Vec::new();
    let mut cycle = None;
    for (index, command) in program.commands.iter().enumerate() {
        if let Command::Loop(None, body) = command {
            if index + 1 != program.commands.len() {
                return Err("無限loopはスクリプトの最後に置いてください".into());
            }
            let mut cycle_frame = frame.clone();
            let mut cycle_actions = Vec::new();
            compile_commands(
                body,
                profile,
                assets,
                values,
                &mut cycle_frame,
                &mut cycle_actions,
                font.as_ref(),
            )?;
            cycle = Some(cycle_actions);
        } else {
            compile_commands(
                std::slice::from_ref(command),
                profile,
                assets,
                values,
                &mut frame,
                &mut actions,
                font.as_ref(),
            )?;
        }
    }
    Ok(ScriptRunner::new(
        profile.profile.canvas_width,
        profile.profile.canvas_height,
        actions,
        cycle,
    ))
}

fn load_font(profile: &Profile, data_root: &Path) -> Result<Option<Arc<BdfFont>>, String> {
    let Some(defaults) = &profile.scroll_defaults else {
        return Ok(None);
    };
    let path = data_root.join("fonts").join(&defaults.font);
    let mut font = BdfFont::parse_bdf(
        &std::fs::read_to_string(path.join("shnmk16.bdf")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    font.merge_fallback(
        BdfFont::parse_bdf(
            &std::fs::read_to_string(path.join("shnm8x16a.bdf")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?,
    );
    Ok(Some(Arc::new(font)))
}

fn compile_commands(
    commands: &[Command],
    profile: &Profile,
    assets: &AssetRegistry,
    values: &serde_json::Map<String, serde_json::Value>,
    frame: &mut RgbFrame,
    actions: &mut Vec<ScriptAction>,
    font: Option<&Arc<BdfFont>>,
) -> Result<(), String> {
    for command in commands {
        match command {
            Command::Frame(ops) => {
                compile_frame(ops, profile, assets, values, frame, actions, font)?
            }
            Command::Loop(Some(n), body) => {
                for _ in 0..*n {
                    compile_commands(body, profile, assets, values, frame, actions, font)?;
                }
            }
            Command::Loop(None, _) => {
                return Err("無限loopはトップレベルの最後だけに置けます".into());
            }
            Command::WhileScroll(body) => {
                let mut body_frame = frame.clone();
                let mut body_actions = Vec::new();
                compile_commands(
                    body,
                    profile,
                    assets,
                    values,
                    &mut body_frame,
                    &mut body_actions,
                    font,
                )?;
                *frame = body_frame;
                actions.push(ScriptAction::WhileScroll(Arc::new(body_actions)));
            }
            Command::CheckScroll => actions.push(ScriptAction::CheckScroll),
            Command::WaitSeconds(value) => {
                actions.push(ScriptAction::Wait(Duration::from_secs_f64(*value)))
            }
            Command::WaitField(field) => actions.push(ScriptAction::Wait(Duration::from_secs_f64(
                number(values, field)?,
            ))),
            Command::WaitScrollEnd => {
                if profile
                    .scroll_defaults
                    .as_ref()
                    .is_some_and(|defaults| defaults.repeat)
                {
                    return Err("repeat=true のスクロールに wait scroll_end は使えません".into());
                }
                actions.push(ScriptAction::WaitScrollEnd)
            }
            Command::Brightness(value) => actions.push(ScriptAction::Brightness(*value)),
            Command::Blank => {
                *frame = RgbFrame::black(frame.width(), frame.height());
                actions.push(ScriptAction::Blank);
            }
            Command::Scroll(region, field) => compile_frame(
                &[FrameOp::Scroll(region.clone(), field.clone())],
                profile,
                assets,
                values,
                frame,
                actions,
                font,
            )?,
        }
    }
    Ok(())
}

fn compile_frame(
    ops: &[FrameOp],
    profile: &Profile,
    assets: &AssetRegistry,
    values: &serde_json::Map<String, serde_json::Value>,
    frame: &mut RgbFrame,
    actions: &mut Vec<ScriptAction>,
    font: Option<&Arc<BdfFont>>,
) -> Result<(), String> {
    let mut next = frame.clone();
    let mut scroll = None;
    for op in ops {
        match op {
            FrameOp::Clear(id) => {
                let r = region(profile, id)?;
                next.clear_region(r.x, r.y, r.width, r.height);
            }
            FrameOp::Set(region_id, field_id) => {
                let r = region(profile, region_id)?;
                let field = profile
                    .fields
                    .iter()
                    .find(|f| f.id == *field_id)
                    .ok_or_else(|| format!("不明なフィールド {field_id}"))?;
                let dir = field
                    .asset_dir
                    .as_deref()
                    .ok_or_else(|| format!("{field_id} は画像フィールドではありません"))?;
                let id = text(values, field_id)?;
                let (w, h, pixels) = assets.load_rgb(dir, id).map_err(|e| e.to_string())?;
                if (w, h) != (r.width, r.height) {
                    return Err(format!(
                        "{field_id}: 期待サイズ {}x{}、実際 {w}x{h}",
                        r.width, r.height
                    ));
                }
                next.blit_rgb(r.x as isize, r.y as isize, w, h, &pixels)
                    .map_err(|e| e.to_string())?;
            }
            FrameOp::Scroll(region_id, field_id) => {
                let defaults = profile
                    .scroll_defaults
                    .as_ref()
                    .ok_or("scroll_defaultsがありません")?;
                let color = color(&defaults.color)?;
                scroll = Some(ScrollSpec {
                    region: region(profile, region_id)?.clone(),
                    text: text(values, field_id)?.to_owned(),
                    font: font.ok_or("スクロールフォントを読み込めません")?.clone(),
                    color,
                    speed_px_per_second: values
                        .get("scroll_speed")
                        .and_then(serde_json::Value::as_f64)
                        .or_else(|| {
                            values
                                .get("scroll_speed")
                                .and_then(serde_json::Value::as_str)
                                .and_then(|value| value.parse().ok())
                        })
                        .unwrap_or(defaults.speed_px_per_second),
                    start_padding: defaults.start_padding,
                    end_padding: defaults.end_padding,
                    repeat: defaults.repeat,
                });
            }
        }
    }
    *frame = next.clone();
    actions.push(ScriptAction::Present {
        frame: next,
        scroll,
    });
    Ok(())
}
fn region<'a>(profile: &'a Profile, id: &str) -> Result<&'a crate::Region, String> {
    profile
        .regions
        .get(id)
        .ok_or_else(|| format!("不明な領域 {id}"))
}
fn text<'a>(
    values: &'a serde_json::Map<String, serde_json::Value>,
    id: &str,
) -> Result<&'a str, String> {
    values
        .get(id)
        .and_then(serde_json::Value::as_str)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("{id} が未選択です"))
}
fn number(values: &serde_json::Map<String, serde_json::Value>, id: &str) -> Result<f64, String> {
    values
        .get(id)
        .and_then(serde_json::Value::as_f64)
        .or_else(|| {
            values
                .get(id)
                .and_then(serde_json::Value::as_str)
                .and_then(|v| v.parse().ok())
        })
        .filter(|v| *v >= 0.0)
        .ok_or_else(|| format!("{id} は秒数で指定してください"))
}
fn color(input: &str) -> Result<[u8; 3], String> {
    let s = input
        .strip_prefix('#')
        .ok_or("色は#RRGGBBで指定してください")?;
    if s.len() != 6 {
        return Err("色は#RRGGBBで指定してください".into());
    };
    Ok([
        u8::from_str_radix(&s[0..2], 16).map_err(|_| "色が不正です")?,
        u8::from_str_radix(&s[2..4], 16).map_err(|_| "色が不正です")?,
        u8::from_str_radix(&s[4..6], 16).map_err(|_| "色が不正です")?,
    ])
}
