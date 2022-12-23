pub mod interactive;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use tui::layout::Direction;

use tui_layout::{
    component::{Component, ComponentBase},
    container::list::ContainerList,
    container::search::ContainerSearch,
    container::Container,
    pos::ComponentPos,
    Border, FocusResult,
};

use crate::interactive::*;

#[test]
fn test_tui_layout() -> Result<(), tui_layout::ResizeError> {
    let component_a = Component::new(
        String::from("a"),
        1,
        Box::new(TestComponentWidget::new(false)),
    );
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

    let mut list_vertical =
        ContainerList::new(String::from("vertical"), Direction::Vertical, true, 0, 0);

    let _ = list_vertical.add_component(component_a);
    let _ = list_vertical.add_component(component_b);

    let mut list_horizontal = ContainerList::new(
        String::from("horizontal"),
        Direction::Horizontal,
        true,
        0,
        0,
    );

    let _ = list_horizontal.add_container(Box::new(list_vertical));
    let _ = list_horizontal.add_component(component_c);

    assert_ne!(list_horizontal.resize(20, 0), Ok(()));
    assert_ne!(list_horizontal.resize(0, 0), Ok(()));
    assert_ne!(list_horizontal.resize(20, 1), Ok(()));
    assert_ne!(list_horizontal.resize(1, 20), Ok(()));

    list_horizontal.resize(20, 10)?;
    list_horizontal.resize(32, 8)?;

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

    let buffer = render_helper(list_horizontal.as_base_mut());
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

    let (comp, pos) = list_horizontal
        .as_container()
        .search_name("c".split(".").map(|s| String::from(s)).collect())
        .unwrap();
    assert_eq!(comp.as_base().get_name(), String::from("c"));
    assert_eq!(pos, ComponentPos { x: 16, y: 0 });

    let (comp, pos) = list_horizontal
        .as_container()
        .search_name("vertical.a".split(".").map(|s| String::from(s)).collect())
        .unwrap();
    assert_eq!(comp.as_base().get_name(), String::from("a"));
    assert_eq!(pos, ComponentPos { x: 0, y: 0 });

    let (comp, pos) = list_horizontal
        .as_container()
        .search_name("vertical.b".split(".").map(|s| String::from(s)).collect())
        .unwrap();
    assert_eq!(comp.as_base().get_name(), String::from("b"));
    assert_eq!(pos, ComponentPos { x: 0, y: 4 });

    if let Some(_) = list_horizontal
        .as_container()
        .search_name("".split(".").map(|s| String::from(s)).collect())
    {
        panic!("<empty> does not exist!");
    }

    if let Some(_) = list_horizontal
        .as_container()
        .search_name("vertical.c".split(".").map(|s| String::from(s)).collect())
    {
        panic!("vertical.c does not exist!");
    }

    if let Some(_) = list_horizontal
        .as_container()
        .search_name("vertical.c".split(".").map(|s| String::from(s)).collect())
    {
        panic!("vertical.b.c does not exist!");
    }

    let (comp, pos) = list_horizontal
        .as_container()
        .search_position(ComponentPos { x: 16, y: 0 })
        .unwrap();
    assert_eq!(comp.get_name(), String::from("c"));
    assert_eq!(pos, ComponentPos { x: 16, y: 0 });

    let (comp, pos) = list_horizontal
        .as_container()
        .search_position(ComponentPos { x: 0, y: 0 })
        .unwrap();
    assert_eq!(comp.get_name(), String::from("a"));
    assert_eq!(pos, ComponentPos { x: 0, y: 0 });

    let (comp, pos) = list_horizontal
        .as_container()
        .search_position(ComponentPos { x: 0, y: 4 })
        .unwrap();
    assert_eq!(comp.get_name(), String::from("b"));
    assert_eq!(pos, ComponentPos { x: 0, y: 4 });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus(_) => panic!("No component should be partial focused!"),
        FocusResult::None => {}
    }

    // Hit enter to partial focus first component
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("a"));
            assert_eq!(pos, ComponentPos { x: 0, y: 0 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit enter to focus
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("a"));
            assert_eq!(pos, ComponentPos { x: 0, y: 0 });
        }
        FocusResult::PartialFocus(_) => panic!("No component should be partial focused!"),
        FocusResult::None => panic!("A component should be focused!"),
    }

    // Hit esc to partial focus
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("a"));
            assert_eq!(pos, ComponentPos { x: 0, y: 0 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit down to partial focus component below
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Down,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("b"));
            assert_eq!(pos, ComponentPos { x: 0, y: 4 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit right to partial focus component right
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Right,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("c"));
            assert_eq!(pos, ComponentPos { x: 16, y: 0 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit right to partial focus component right (should not change)
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Right,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("c"));
            assert_eq!(pos, ComponentPos { x: 16, y: 0 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    assert_eq!(list_horizontal.get_border(1, 0), Some(Border::Top));
    assert_eq!(list_horizontal.get_border(0, 1), Some(Border::Left));
    assert_eq!(list_horizontal.get_border(0, 6), Some(Border::Left));
    assert_eq!(list_horizontal.get_border(15, 1), None);
    assert_eq!(list_horizontal.get_border(1, 7), Some(Border::Bottom));
    assert_eq!(list_horizontal.get_border(31, 1), Some(Border::Right));

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

    list_horizontal.handle_mouse(16, 4, Some(MouseEventKind::Down(MouseButton::Left)));
    list_horizontal.handle_mouse(17, 4, Some(MouseEventKind::Drag(MouseButton::Left)));
    list_horizontal.handle_mouse(17, 4, None);
    list_horizontal.handle_mouse(18, 4, Some(MouseEventKind::Drag(MouseButton::Left)));

    let buffer = render_helper(list_horizontal.as_base_mut());
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
fn test_tui() -> Result<(), std::io::Error> {
    tui_main()
}