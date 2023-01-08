pub mod interactive;

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton, MouseEventKind,
};
use tui::layout::Direction;
use tui_tiling::{
    component::{Component, ComponentBase},
    container::list::ContainerList,
    container::search::ContainerSearch,
    container::Container,
    pos::ComponentPos,
    Border, FocusResult,
};

use crate::interactive::*;

#[test]
fn test_tui_tiling() -> Result<(), tui_tiling::ResizeError> {
    let mut component_a = Component::new(
        String::from("a"),
        1,
        Box::new(TestComponentWidget::new(false)),
    );
    component_a.set_fixed_height(Some(4));
    let component_b = Component::new(
        String::from("b"),
        1,
        Box::new(TestComponentWidget::new(false)),
    );
    let component_c = Component::new(
        String::from("c"),
        1,
        Box::new(TestComponentWidget::new(false)),
    );

    assert!(component_a
        .get_widget()
        .as_any()
        .downcast_ref::<TestComponentWidget>()
        .is_some());

    assert!(component_a
        .get_widget_mut()
        .as_any_mut()
        .downcast_mut::<TestComponentWidget>()
        .is_some());

    let mut list_vertical =
        ContainerList::new(String::from("vertical"), Direction::Vertical, true, 0, 0);

    let _ = list_vertical.add_component(component_a);
    let _ = list_vertical.add_component(component_b);

    let mut tui = ContainerList::new(
        String::from("horizontal"),
        Direction::Horizontal,
        true,
        0,
        0,
    );

    let _ = tui.add_container(Box::new(list_vertical));
    let _ = tui.add_component(component_c);

    assert!(tui
        .as_container()
        .search_name_widget::<TestComponentWidget>("vertical.a")
        .is_some());

    assert!(tui
        .as_container_mut()
        .search_name_widget_mut::<TestComponentWidget>("vertical.a")
        .is_some());

    assert_ne!(tui.resize(20, 0), Ok(()));
    // Size should still be zero since last change was un-done
    assert_eq!(tui.resize(0, 0), Ok(()));
    assert_ne!(tui.resize(20, 1), Ok(()));
    assert_ne!(tui.resize(1, 20), Ok(()));

    tui.resize(20, 10)?;
    tui.resize(32, 8)?;

    let expected = [
        "╭a─────────────╮╭c─────────────╮",
        "│##############││##############│",
        "│##############││#            #│",
        "╰──────────────╯│#            #│",
        "╭b─────────────╮│#            #│",
        "│##############││#            #│",
        "│##############││##############│",
        "╰──────────────╯╰──────────────╯",
    ];

    let buffer = render_helper(tui.as_base_mut());
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            print!("{}", buffer.get(x, y).symbol);
        }
        println!();
    }
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            assert_eq!(
                buffer.get(x, y).symbol.chars().nth(0).unwrap(),
                expected[y as usize].chars().nth(x as usize).unwrap()
            );
        }
    }

    let (comp, pos) = tui.as_container().search_name("c").unwrap();
    assert_eq!(comp.as_base().get_name(), String::from("c"));
    assert_eq!(pos, ComponentPos { x: 16, y: 0 });

    let (comp, pos) = tui.as_container().search_name("vertical.a").unwrap();
    assert_eq!(comp.as_base().get_name(), String::from("a"));
    assert_eq!(pos, ComponentPos { x: 0, y: 0 });

    let (comp, pos) = tui.as_container().search_name("vertical.b").unwrap();
    assert_eq!(comp.as_base().get_name(), String::from("b"));
    assert_eq!(pos, ComponentPos { x: 0, y: 4 });

    if let Some(_) = tui.as_container().search_name("") {
        panic!("<empty> does not exist!");
    }

    if let Some(_) = tui.as_container().search_name("vertical.c") {
        panic!("vertical.c does not exist!");
    }

    if let Some(_) = tui.as_container().search_name("vertical.c") {
        panic!("vertical.b.c does not exist!");
    }

    let (comp, pos) = tui
        .as_container()
        .search_position(ComponentPos { x: 16, y: 0 })
        .unwrap();
    assert_eq!(comp.get_name(), String::from("c"));
    assert_eq!(pos, ComponentPos { x: 16, y: 0 });

    let (comp, pos) = tui
        .as_container()
        .search_position(ComponentPos { x: 0, y: 0 })
        .unwrap();
    assert_eq!(comp.get_name(), String::from("a"));
    assert_eq!(pos, ComponentPos { x: 0, y: 0 });

    let (comp, pos) = tui
        .as_container()
        .search_position(ComponentPos { x: 0, y: 4 })
        .unwrap();
    assert_eq!(comp.get_name(), String::from("b"));
    assert_eq!(pos, ComponentPos { x: 0, y: 4 });

    match tui.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus(_) => panic!("No component should be partial focused!"),
        FocusResult::None => {}
    }

    // Hit enter to partial focus first component
    tui.handle_key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    match tui.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("a"));
            assert_eq!(pos, ComponentPos { x: 0, y: 0 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit enter to focus
    tui.handle_key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    match tui.as_container().search_focused() {
        FocusResult::Focus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("a"));
            assert_eq!(pos, ComponentPos { x: 0, y: 0 });
        }
        FocusResult::PartialFocus(_) => panic!("No component should be partial focused!"),
        FocusResult::None => panic!("A component should be focused!"),
    }

    // Hit esc to partial focus
    tui.handle_key(KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    match tui.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("a"));
            assert_eq!(pos, ComponentPos { x: 0, y: 0 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit down to partial focus component below
    tui.handle_key(KeyEvent {
        code: KeyCode::Down,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    match tui.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("b"));
            assert_eq!(pos, ComponentPos { x: 0, y: 4 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit right to partial focus component right
    tui.handle_key(KeyEvent {
        code: KeyCode::Right,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    match tui.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("c"));
            assert_eq!(pos, ComponentPos { x: 16, y: 0 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit right to partial focus component right (should not change)
    tui.handle_key(KeyEvent {
        code: KeyCode::Right,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    match tui.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("c"));
            assert_eq!(pos, ComponentPos { x: 16, y: 0 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    assert_eq!(tui.get_border(1, 0), Some(Border::Top));
    assert_eq!(tui.get_border(0, 1), Some(Border::Left));
    assert_eq!(tui.get_border(0, 6), Some(Border::Left));
    assert_eq!(tui.get_border(15, 1), None);
    assert_eq!(tui.get_border(1, 7), Some(Border::Bottom));
    assert_eq!(tui.get_border(31, 1), Some(Border::Right));

    let expected = [
        "╭a──────────────╮╭c────────────╮",
        "│###############││#############│",
        "│###############││#           #│",
        "╰───────────────╯│#           #│",
        "╭b──────────────╮│#           #│",
        "│###############││#           #│",
        "│###############││#############│",
        "╰───────────────╯╰─────────────╯",
    ];

    tui.handle_mouse(16, 4, Some(MouseEventKind::Down(MouseButton::Left)));
    tui.handle_mouse(17, 4, Some(MouseEventKind::Drag(MouseButton::Left)));
    tui.handle_mouse(17, 4, None);
    tui.handle_mouse(18, 4, Some(MouseEventKind::Drag(MouseButton::Left)));

    let buffer = render_helper(tui.as_base_mut());
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            assert_eq!(
                buffer.get(x, y).symbol.chars().nth(0).unwrap(),
                expected[y as usize].chars().nth(x as usize).unwrap()
            );
            print!("{}", buffer.get(x, y).symbol);
        }
        println!();
    }

    Ok(())
}

#[test]
fn test_tui_interactive() -> Result<(), std::io::Error> {
    tui_main()
}
