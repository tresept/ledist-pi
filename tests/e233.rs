use ledist_pi::{E233DisplaySelection, E233Layout, FieldSelection, ScrollCycleItem, plan_e233};

fn selection() -> E233DisplaySelection {
    E233DisplaySelection {
        service: FieldSelection::None,
        route: FieldSelection::None,
        service_change: FieldSelection::None,
        through_route: FieldSelection::None,
        destination: FieldSelection::None,
        next_stop: FieldSelection::None,
        scroll_text: String::new(),
        scroll_speed: 50.0,
        scroll_cycle: vec![
            ScrollCycleItem::DestinationJa,
            ScrollCycleItem::DestinationEn,
        ],
        brightness: 40,
    }
}
#[test]
fn destination_and_service_change_are_separate_pages() {
    let mut s = selection();
    s.service = FieldSelection::Asset("s".into());
    s.destination = FieldSelection::Asset("d".into());
    s.service_change = FieldSelection::Asset("c".into());
    assert_eq!(plan_e233(&s).unwrap().pages.len(), 2)
}
#[test]
fn makes_service_destination_scroll_page() {
    let mut s = selection();
    s.service = FieldSelection::Asset("s".into());
    s.destination = FieldSelection::Asset("d".into());
    s.scroll_text = "next".into();
    let plan = plan_e233(&s).unwrap();
    assert!(matches!(
        plan.pages.last().unwrap().layout,
        E233Layout::ServiceAndRightSplit(_, _, _)
    ));
}

#[test]
fn next_stop_creates_japanese_and_english_split_pages() {
    let mut s = selection();
    s.service = FieldSelection::Asset("local".into());
    s.destination = FieldSelection::Asset("shinjuku".into());
    s.next_stop = FieldSelection::Asset("osaki".into());
    let plan = plan_e233(&s).unwrap();
    assert_eq!(plan.pages.len(), 2);
    assert!(matches!(
        plan.pages[0].layout,
        E233Layout::ServiceAndRightSplit(..)
    ));
    assert!(matches!(
        plan.pages[1].layout,
        E233Layout::ServiceAndRightSplit(..)
    ));
}

#[test]
fn scroll_requires_destination() {
    let selection = E233DisplaySelection {
        service: FieldSelection::Asset("local".into()),
        route: FieldSelection::Asset("saikyo".into()),
        service_change: FieldSelection::None,
        through_route: FieldSelection::None,
        destination: FieldSelection::None,
        next_stop: FieldSelection::None,
        scroll_text: "この電車は相鉄線へ直通します".into(),
        scroll_speed: 50.0,
        scroll_cycle: vec![ScrollCycleItem::DestinationJa],
        brightness: 40,
    };

    assert!(plan_e233(&selection).is_err());
}

#[test]
fn static_pages_follow_destination_route_through_change_order() {
    let mut s = selection();
    s.service = FieldSelection::Blank;
    s.destination = FieldSelection::Asset("d".into());
    s.route = FieldSelection::Asset("r".into());
    s.through_route = FieldSelection::Asset("t".into());
    let plan = plan_e233(&s).unwrap();
    assert_eq!(plan.pages.len(), 3);
    assert!(matches!(
        plan.pages[0].layout,
        E233Layout::ServiceAndRight(..)
    ));
    assert!(matches!(
        plan.pages[1].layout,
        E233Layout::ServiceAndRight(..)
    ));
    assert!(matches!(
        plan.pages[2].layout,
        E233Layout::ServiceAndRight(..)
    ));
}

#[test]
fn rejects_scroll_without_a_cycle_item_or_destination() {
    let mut s = selection();
    s.destination = FieldSelection::Asset("d".into());
    s.scroll_text = "notice".into();
    s.scroll_cycle.clear();
    assert!(plan_e233(&s).is_err());
    let mut s = selection();
    s.scroll_text = "notice".into();
    assert!(plan_e233(&s).is_err());
}
