use crate::{
    AssetRegistry, BdfFont, E233Config, Profile, Region, RgbFrame, ScriptAction, ScriptRunner,
    ScrollSpec,
};
use std::{path::Path, sync::Arc, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldSelection {
    None,
    /// Kept for in-process compatibility. Web requests use an actual PNG such
    /// as `無表示.png` instead of this variant.
    Blank,
    Asset(String),
}
impl FieldSelection {
    pub fn participates(&self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollCycleItem {
    DestinationJa,
    DestinationEn,
    Route,
    ThroughRoute,
}

#[derive(Clone, Debug)]
pub struct DisplaySelection {
    pub service: FieldSelection,
    pub destination: FieldSelection,
    pub route: FieldSelection,
    pub through_route: FieldSelection,
    pub service_change: FieldSelection,
    pub next_stop: FieldSelection,
    pub scroll_text: String,
    pub scroll_speed: f64,
    pub scroll_cycle: Vec<ScrollCycleItem>,
    pub brightness: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Content {
    Blank,
    Field(&'static str, FieldSelection),
    Scroll(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Layout {
    Full(Content),
    ServiceAndRight(Content, Content),
    ServiceAndRightSplit(Content, Content, Content),
    FullWidthSplit(Content, Content),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageDuration {
    Fixed(Duration),
    UntilScrollEnd,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub layout: Layout,
    pub duration: PageDuration,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayPlan {
    pub pages: Vec<Page>,
}

const FIXED: Duration = Duration::from_secs(3);
pub const FULL: Region = Region {
    x: 0,
    y: 0,
    width: 128,
    height: 32,
};
pub const SERVICE_LEFT: Region = Region {
    x: 0,
    y: 0,
    width: 48,
    height: 32,
};
pub const RIGHT_FULL: Region = Region {
    x: 48,
    y: 0,
    width: 80,
    height: 32,
};
pub const RIGHT_TOP: Region = Region {
    x: 48,
    y: 0,
    width: 80,
    height: 16,
};
pub const RIGHT_BOTTOM: Region = Region {
    x: 48,
    y: 16,
    width: 80,
    height: 16,
};
pub const FULL_TOP: Region = Region {
    x: 0,
    y: 0,
    width: 128,
    height: 16,
};
pub const FULL_BOTTOM: Region = Region {
    x: 0,
    y: 16,
    width: 128,
    height: 16,
};

/// Public summary used by validation tests and API diagnostics.
pub fn plan(selection: &DisplaySelection) -> Result<DisplayPlan, String> {
    if !selection.service.participates() && !selection.destination.participates() {
        return Err("種別または行先を選択してください。".into());
    }
    if !selection.service.participates()
        && (selection.route.participates()
            || selection.through_route.participates()
            || selection.service_change.participates())
    {
        return Err("路線名・直通先路線名・種別変更は種別と一緒に表示してください。".into());
    }
    if !selection.scroll_text.trim().is_empty() && !selection.destination.participates() {
        return Err("スクロール文字を表示するには行先を選択してください。".into());
    }
    if !selection.scroll_text.trim().is_empty() && selection.scroll_cycle.is_empty() {
        return Err("スクロール中に切り替える項目を1つ以上選択してください。".into());
    }
    let service = |language| Content::Field(language, selection.service.clone());
    let mut pages = Vec::new();
    if !selection.scroll_text.trim().is_empty() {
        if selection.service.participates() {
            pages.push(Page {
                layout: Layout::ServiceAndRightSplit(
                    service("service_ja"),
                    Content::Field("destination_ja", selection.destination.clone()),
                    Content::Scroll(selection.scroll_text.clone()),
                ),
                duration: PageDuration::UntilScrollEnd,
            });
        } else {
            pages.push(Page {
                layout: Layout::FullWidthSplit(
                    Content::Field("destination_full_top", selection.destination.clone()),
                    Content::Scroll(selection.scroll_text.clone()),
                ),
                duration: PageDuration::UntilScrollEnd,
            });
        }
        return Ok(DisplayPlan { pages });
    }
    if selection.service.participates() {
        if selection.destination.participates() {
            if selection.next_stop.participates() {
                pages.push(Page {
                    layout: Layout::ServiceAndRightSplit(
                        service("service_ja"),
                        Content::Field("destination_ja", selection.destination.clone()),
                        Content::Field("next_stop_ja", selection.next_stop.clone()),
                    ),
                    duration: PageDuration::Fixed(FIXED),
                });
                pages.push(Page {
                    layout: Layout::ServiceAndRightSplit(
                        service("service_en"),
                        Content::Field("destination_en", selection.destination.clone()),
                        Content::Field("next_stop_en", selection.next_stop.clone()),
                    ),
                    duration: PageDuration::Fixed(FIXED),
                });
            } else {
                pages.push(Page {
                    layout: Layout::ServiceAndRight(
                        service("service_ja"),
                        Content::Field("destination_right", selection.destination.clone()),
                    ),
                    duration: PageDuration::Fixed(FIXED),
                });
            }
        }
        if selection.route.participates() && selection.through_route.participates() {
            pages.push(Page {
                layout: Layout::ServiceAndRightSplit(
                    service("service_ja"),
                    Content::Field("route_top", selection.route.clone()),
                    Content::Field("through_route_bottom", selection.through_route.clone()),
                ),
                duration: PageDuration::Fixed(FIXED),
            });
        } else if selection.route.participates() || selection.through_route.participates() {
            let (name, value) = if selection.route.participates() {
                ("route_right", &selection.route)
            } else {
                ("through_route_right", &selection.through_route)
            };
            pages.push(Page {
                layout: Layout::ServiceAndRight(
                    service("service_ja"),
                    Content::Field(name, value.clone()),
                ),
                duration: PageDuration::Fixed(FIXED),
            });
        }
        if selection.service_change.participates() {
            pages.push(Page {
                layout: Layout::ServiceAndRight(
                    service("service_ja"),
                    Content::Field("service_change_right", selection.service_change.clone()),
                ),
                duration: PageDuration::Fixed(FIXED),
            });
        }
        if pages.is_empty() {
            pages.push(Page {
                layout: Layout::Full(Content::Field("service_full", selection.service.clone())),
                duration: PageDuration::Fixed(FIXED),
            });
        }
    } else if selection.next_stop.participates() {
        pages.push(Page {
            layout: Layout::FullWidthSplit(
                Content::Field("destination_full_top", selection.destination.clone()),
                Content::Field("next_stop_full_bottom", selection.next_stop.clone()),
            ),
            duration: PageDuration::Fixed(FIXED),
        });
    } else {
        pages.push(Page {
            layout: Layout::Full(Content::Field(
                "destination_full",
                selection.destination.clone(),
            )),
            duration: PageDuration::Fixed(FIXED),
        });
    }
    Ok(DisplayPlan { pages })
}

pub fn compile(
    profile: &Profile,
    assets: &AssetRegistry,
    selection: &DisplaySelection,
    data_root: &Path,
) -> Result<ScriptRunner, String> {
    plan(selection)?;
    let config = profile.e233.as_ref().ok_or("E233設定がありません")?;
    let duration = Duration::from_secs_f64(config.page_seconds);
    if should_use_service_full(assets, config, selection) {
        let frame = full_value(
            profile,
            assets,
            config,
            "service",
            &selection.service,
            "full",
        )?;
        return Ok(ScriptRunner::new(
            profile.profile.canvas_width,
            profile.profile.canvas_height,
            Vec::new(),
            Some(vec![
                ScriptAction::Present {
                    frame,
                    scroll: None,
                },
                ScriptAction::Wait(duration),
            ]),
        ));
    }
    let mut actions = Vec::new();
    if selection.scroll_text.trim().is_empty() {
        for page in normal_pages(profile, assets, config, selection)? {
            actions.push(ScriptAction::Present {
                frame: page,
                scroll: None,
            });
            actions.push(ScriptAction::Wait(duration));
        }
    } else {
        let font = load_font(profile, data_root)?;
        let start = scroll_frame(
            profile,
            assets,
            config,
            selection,
            &ScrollCycleItem::DestinationJa,
        )?;
        let region = if selection.service.participates() {
            RIGHT_BOTTOM
        } else {
            FULL_BOTTOM
        };
        let mut spec = scroll_spec(
            profile,
            Some(&font),
            selection.scroll_text.trim().to_owned(),
            region,
        )?;
        spec.speed_px_per_second = selection.scroll_speed;
        actions.push(ScriptAction::Present {
            frame: start,
            scroll: Some(spec),
        });
        actions.push(ScriptAction::Wait(duration));
        let mut body = Vec::new();
        for item in canonical_cycle(&selection.scroll_cycle) {
            if let Some(frame) = scroll_cycle_frame(profile, assets, config, selection, &item)? {
                body.push(ScriptAction::Present {
                    frame,
                    scroll: None,
                });
                body.push(ScriptAction::Wait(duration));
            }
        }
        if body.is_empty() {
            return Err("スクロール中に表示できる切替項目がありません。".into());
        }
        actions.push(ScriptAction::WhileScroll(Arc::new(body)));
        actions.push(ScriptAction::WaitScrollEnd);
        for frame in post_scroll_pages(profile, assets, config, selection)? {
            actions.push(ScriptAction::Present {
                frame,
                scroll: None,
            });
            actions.push(ScriptAction::Wait(duration));
        }
    }
    Ok(ScriptRunner::new(
        profile.profile.canvas_width,
        profile.profile.canvas_height,
        Vec::new(),
        Some(actions),
    ))
}

/// Some special service types (for example 回送 and 試運転) only have a
/// 128×32 service curtain.  A complete 48×32 Japanese/English pair is
/// required before the normal split layouts are allowed.
fn should_use_service_full(
    assets: &AssetRegistry,
    config: &E233Config,
    selection: &DisplaySelection,
) -> bool {
    let FieldSelection::Asset(id) = &selection.service else {
        return false;
    };
    selection.scroll_text.trim().is_empty()
        && !selection.destination.participates()
        && !selection.route.participates()
        && !selection.through_route.participates()
        && !selection.service_change.participates()
        && !selection.next_stop.participates()
        && has_asset(assets, config, "service", id, "full", FULL)
}

fn has_asset(
    assets: &AssetRegistry,
    config: &E233Config,
    group: &str,
    id: &str,
    slot: &str,
    region: Region,
) -> bool {
    config
        .assets
        .get(group)
        .and_then(|group| group.directories.get(slot))
        .into_iter()
        .flatten()
        .any(|directory| {
            assets
                .load_rgb(directory, id)
                .is_ok_and(|(width, height, _)| (width, height) == (region.width, region.height))
        })
}

fn normal_pages(
    profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    s: &DisplaySelection,
) -> Result<Vec<RgbFrame>, String> {
    let mut pages = Vec::new();
    if s.service.participates() {
        if s.destination.participates() {
            if s.next_stop.participates() {
                pages.push(split(
                    profile,
                    assets,
                    config,
                    s,
                    "left-ja",
                    "right-top-ja",
                    "right-bottom-ja",
                    "destination",
                    "next_stop",
                )?);
                pages.push(split(
                    profile,
                    assets,
                    config,
                    s,
                    "left-en",
                    "right-top-en",
                    "right-bottom-en",
                    "destination",
                    "next_stop",
                )?);
            } else {
                pages.push(right(
                    profile,
                    assets,
                    config,
                    s,
                    "left-ja",
                    "destination",
                    "right",
                )?);
            }
        }
        if s.route.participates() && s.through_route.participates() {
            pages.push(route_through_frame(profile, assets, config, s)?);
        } else if s.route.participates() || s.through_route.participates() {
            let (group, value) = if s.route.participates() {
                ("route", &s.route)
            } else {
                ("through_route", &s.through_route)
            };
            pages.push(right_value(
                profile, assets, config, &s.service, "left-ja", group, value, "right",
            )?);
        }
        if s.service_change.participates() {
            pages.push(right_value(
                profile,
                assets,
                config,
                &s.service,
                "left-ja",
                "service_change",
                &s.service_change,
                "right",
            )?);
        }
        if pages.is_empty() {
            pages.push(service_single(profile, assets, config, &s.service)?);
        }
    } else if s.next_stop.participates() {
        pages.push(full_split(profile, assets, config, s)?);
    } else {
        pages.push(destination_single(profile, assets, config, &s.destination)?);
    }
    Ok(pages)
}

/// Pages shown after a scroll has completed.  The initial destination-only
/// pages have already been shown before the scroll, so only the optional
/// next-stop, route/through-route, and service-change pages belong here.
fn post_scroll_pages(
    profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    s: &DisplaySelection,
) -> Result<Vec<RgbFrame>, String> {
    if !s.service.participates() {
        return if s.next_stop.participates() {
            Ok(vec![full_split(profile, assets, config, s)?])
        } else {
            Ok(Vec::new())
        };
    }
    let mut pages = Vec::new();
    if s.next_stop.participates() {
        pages.push(split(
            profile,
            assets,
            config,
            s,
            "left-ja",
            "right-top-ja",
            "right-bottom-ja",
            "destination",
            "next_stop",
        )?);
        pages.push(split(
            profile,
            assets,
            config,
            s,
            "left-en",
            "right-top-en",
            "right-bottom-en",
            "destination",
            "next_stop",
        )?);
    }
    if s.route.participates() && s.through_route.participates() {
        pages.push(route_through_frame(profile, assets, config, s)?);
    } else if s.route.participates() || s.through_route.participates() {
        let (group, value) = if s.route.participates() {
            ("route", &s.route)
        } else {
            ("through_route", &s.through_route)
        };
        pages.push(right_value(
            profile, assets, config, &s.service, "left-ja", group, value, "right",
        )?);
    }
    if s.service_change.participates() {
        pages.push(right_value(
            profile,
            assets,
            config,
            &s.service,
            "left-ja",
            "service_change",
            &s.service_change,
            "right",
        )?);
    }
    Ok(pages)
}

fn service_single(
    profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    service: &FieldSelection,
) -> Result<RgbFrame, String> {
    if let FieldSelection::Asset(id) = service
        && has_asset(assets, config, "service", id, "full", FULL)
    {
        return full_value(profile, assets, config, "service", service, "full");
    }
    let mut frame = RgbFrame::black(128, 32);
    draw(
        profile,
        assets,
        config,
        "service",
        service,
        "left-ja",
        SERVICE_LEFT,
        &mut frame,
    )?;
    Ok(frame)
}

fn destination_single(
    profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    destination: &FieldSelection,
) -> Result<RgbFrame, String> {
    let FieldSelection::Asset(id) = destination else {
        return Err("行先を選択してください。".into());
    };
    if has_asset(assets, config, "destination", id, "full", FULL) {
        return full_value(profile, assets, config, "destination", destination, "full");
    }
    let mut frame = RgbFrame::black(128, 32);
    if has_asset(assets, config, "destination", id, "right", RIGHT_FULL) {
        draw(
            profile,
            assets,
            config,
            "destination",
            destination,
            "right",
            RIGHT_FULL,
            &mut frame,
        )?;
        return Ok(frame);
    }
    if has_asset(assets, config, "destination", id, "right-top-ja", RIGHT_TOP) {
        draw(
            profile,
            assets,
            config,
            "destination",
            destination,
            "right-top-ja",
            RIGHT_TOP,
            &mut frame,
        )?;
        return Ok(frame);
    }
    Err(format!(
        "行先「{id}」には128x32、80x32、80x16のいずれの素材もありません。"
    ))
}

fn canonical_cycle(items: &[ScrollCycleItem]) -> Vec<ScrollCycleItem> {
    [
        ScrollCycleItem::DestinationJa,
        ScrollCycleItem::DestinationEn,
        ScrollCycleItem::Route,
        ScrollCycleItem::ThroughRoute,
    ]
    .into_iter()
    .filter(|item| items.contains(item))
    .collect()
}
fn scroll_cycle_frame(
    profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    s: &DisplaySelection,
    item: &ScrollCycleItem,
) -> Result<Option<RgbFrame>, String> {
    match item {
        ScrollCycleItem::Route if !s.route.participates() => Ok(None),
        ScrollCycleItem::ThroughRoute if !s.through_route.participates() => Ok(None),
        _ => scroll_frame(profile, assets, config, s, item).map(Some),
    }
}
fn scroll_frame(
    profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    s: &DisplaySelection,
    item: &ScrollCycleItem,
) -> Result<RgbFrame, String> {
    if !s.service.participates() {
        return full_value(
            profile,
            assets,
            config,
            "destination",
            &s.destination,
            "full-top",
        );
    }
    let (service_slot, group, value, slot) = match item {
        ScrollCycleItem::DestinationJa => {
            ("left-ja", "destination", &s.destination, "right-top-ja")
        }
        ScrollCycleItem::DestinationEn => {
            ("left-en", "destination", &s.destination, "right-top-en")
        }
        ScrollCycleItem::Route => ("left-ja", "route", &s.route, "right-top"),
        ScrollCycleItem::ThroughRoute => {
            ("left-ja", "through_route", &s.through_route, "right-top")
        }
    };
    let mut frame = RgbFrame::black(128, 32);
    draw(
        profile,
        assets,
        config,
        "service",
        &s.service,
        service_slot,
        SERVICE_LEFT,
        &mut frame,
    )?;
    draw(
        profile, assets, config, group, value, slot, RIGHT_TOP, &mut frame,
    )?;
    Ok(frame)
}
fn right(
    profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    s: &DisplaySelection,
    service_slot: &str,
    group: &str,
    slot: &str,
) -> Result<RgbFrame, String> {
    right_value(
        profile,
        assets,
        config,
        &s.service,
        service_slot,
        group,
        &s.destination,
        slot,
    )
}
fn right_value(
    profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    service: &FieldSelection,
    service_slot: &str,
    group: &str,
    value: &FieldSelection,
    slot: &str,
) -> Result<RgbFrame, String> {
    let mut frame = RgbFrame::black(128, 32);
    draw(
        profile,
        assets,
        config,
        "service",
        service,
        service_slot,
        SERVICE_LEFT,
        &mut frame,
    )?;
    draw(
        profile, assets, config, group, value, slot, RIGHT_FULL, &mut frame,
    )?;
    Ok(frame)
}

fn route_through_frame(
    profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    s: &DisplaySelection,
) -> Result<RgbFrame, String> {
    let mut frame = RgbFrame::black(128, 32);
    draw(
        profile,
        assets,
        config,
        "service",
        &s.service,
        "left-ja",
        SERVICE_LEFT,
        &mut frame,
    )?;
    draw(
        profile,
        assets,
        config,
        "route",
        &s.route,
        "right-top",
        RIGHT_TOP,
        &mut frame,
    )?;
    draw(
        profile,
        assets,
        config,
        "through_route",
        &s.through_route,
        "right-top",
        RIGHT_BOTTOM,
        &mut frame,
    )?;
    Ok(frame)
}

fn split(
    profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    s: &DisplaySelection,
    service_slot: &str,
    destination_slot: &str,
    next_slot: &str,
    destination_group: &str,
    next_group: &str,
) -> Result<RgbFrame, String> {
    let mut frame = RgbFrame::black(128, 32);
    draw(
        profile,
        assets,
        config,
        "service",
        &s.service,
        service_slot,
        SERVICE_LEFT,
        &mut frame,
    )?;
    draw(
        profile,
        assets,
        config,
        destination_group,
        &s.destination,
        destination_slot,
        RIGHT_TOP,
        &mut frame,
    )?;
    draw(
        profile,
        assets,
        config,
        next_group,
        &s.next_stop,
        next_slot,
        RIGHT_BOTTOM,
        &mut frame,
    )?;
    Ok(frame)
}
fn full_split(
    profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    s: &DisplaySelection,
) -> Result<RgbFrame, String> {
    let mut frame = RgbFrame::black(128, 32);
    draw(
        profile,
        assets,
        config,
        "destination",
        &s.destination,
        "full-top",
        FULL_TOP,
        &mut frame,
    )?;
    draw(
        profile,
        assets,
        config,
        "next_stop",
        &s.next_stop,
        "full-bottom-ja",
        FULL_BOTTOM,
        &mut frame,
    )?;
    Ok(frame)
}
fn full_value(
    profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    group: &str,
    value: &FieldSelection,
    slot: &str,
) -> Result<RgbFrame, String> {
    let mut frame = RgbFrame::black(128, 32);
    let region = if slot == "full" { FULL } else { FULL_TOP };
    draw(
        profile, assets, config, group, value, slot, region, &mut frame,
    )?;
    Ok(frame)
}
fn draw(
    _profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    group: &str,
    value: &FieldSelection,
    slot: &str,
    region: Region,
    frame: &mut RgbFrame,
) -> Result<(), String> {
    let FieldSelection::Asset(id) = value else {
        return Ok(());
    };
    let group_config = config
        .assets
        .get(group)
        .ok_or_else(|| format!("{group} の素材設定がありません"))?;
    let mut slots = vec![slot];
    if let Some(alternate) = alternate_language_slot(slot) {
        slots.push(alternate);
    }
    for candidate in slots {
        if let Some(directories) = group_config.directories.get(candidate) {
            for directory in directories {
                if let Ok((w, h, pixels)) = assets.load_rgb(directory, id)
                    && (w, h) == (region.width, region.height)
                {
                    frame
                        .blit_rgb(region.x as isize, region.y as isize, w, h, &pixels)
                        .map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }
        }
    }
    if alternate_language_slot(slot).is_some() {
        return Ok(());
    }
    let directories = group_config
        .directories
        .get(slot)
        .ok_or_else(|| format!("{group} に {slot} 用素材設定がありません"))?;
    Err(format!(
        "{group} の「{id}」に {}x{} 素材がありません（{}）",
        region.width,
        region.height,
        directories.join(" → ")
    ))
}

fn alternate_language_slot(slot: &str) -> Option<&'static str> {
    match slot {
        "left-ja" => Some("left-en"),
        "left-en" => Some("left-ja"),
        "right-top-ja" => Some("right-top-en"),
        "right-top-en" => Some("right-top-ja"),
        "right-bottom-ja" => Some("right-bottom-en"),
        "right-bottom-en" => Some("right-bottom-ja"),
        "full-bottom-ja" => Some("full-bottom-en"),
        "full-bottom-en" => Some("full-bottom-ja"),
        _ => None,
    }
}
fn load_font(profile: &Profile, data_root: &Path) -> Result<Arc<BdfFont>, String> {
    let defaults = profile
        .scroll_defaults
        .as_ref()
        .ok_or("scroll_defaultsがありません")?;
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
    Ok(Arc::new(font))
}
pub(crate) fn scroll_spec(
    profile: &Profile,
    font: Option<&Arc<BdfFont>>,
    text: String,
    region: Region,
) -> Result<ScrollSpec, String> {
    let defaults = profile
        .scroll_defaults
        .as_ref()
        .ok_or("scroll_defaultsがありません")?;
    let color = defaults
        .color
        .strip_prefix('#')
        .ok_or("色は#RRGGBBで指定してください")?;
    if color.len() != 6 {
        return Err("色は#RRGGBBで指定してください".into());
    }
    Ok(ScrollSpec {
        region,
        text,
        font: font.ok_or("スクロールフォントを読み込めません")?.clone(),
        color: [
            u8::from_str_radix(&color[0..2], 16).map_err(|_| "色が不正です")?,
            u8::from_str_radix(&color[2..4], 16).map_err(|_| "色が不正です")?,
            u8::from_str_radix(&color[4..6], 16).map_err(|_| "色が不正です")?,
        ],
        speed_px_per_second: defaults.speed_px_per_second,
        start_padding: defaults.start_padding,
        end_padding: defaults.end_padding,
        repeat: false,
    })
}
