use tui::layout::Rect;

use crate::{
    component::{Component, ComponentBase, ComponentWidget},
    container::{Container, ContainerChild},
    pos::ComponentPos,
    Focus, FocusResult,
};

pub trait ContainerSearch {
    fn search_focused(&self) -> FocusResult<(&Component, ComponentPos)>;
    fn search_focused_mut(&mut self) -> FocusResult<(&mut Component, ComponentPos)>;

    fn search_position(&self, pos: ComponentPos) -> Option<(&Component, ComponentPos)>;
    fn search_position_mut(&mut self, pos: ComponentPos) -> Option<(&mut Component, ComponentPos)>;

    fn search_name(&self, path: &str) -> Option<(&ContainerChild, ComponentPos)>;
    fn search_name_mut(&mut self, path: &str) -> Option<(&mut ContainerChild, ComponentPos)>;

    fn search_name_widget<T>(&self, path: &str) -> Option<&T>
    where
        T: ComponentWidget + 'static,
    {
        let Some((child, _)) = self.search_name(path) else {
            return None;
        };
        let ContainerChild::Component(component) = child else {
            return None;
        };
        let Some(widget) = component.get_widget().as_any().downcast_ref::<T>() else {
            return None;
        };
        Some(widget)
    }

    fn search_name_widget_mut<T>(&mut self, path: &str) -> Option<&mut T>
    where
        T: ComponentWidget + 'static,
    {
        let Some((child, _)) = self.search_name_mut(path) else {
            return None;
        };
        let ContainerChild::Component(component) = child else {
            return None;
        };
        let Some(widget) = component.get_widget_mut().as_any_mut().downcast_mut::<T>() else {
            return None;
        };
        Some(widget)
    }
}

fn get_positions(container: &dyn Container) -> Vec<ComponentPos> {
    container
        .get_children_rectangles()
        .iter()
        .map(|r| ComponentPos { x: r.x, y: r.y })
        .collect::<Vec<ComponentPos>>()
}

impl<'a> ContainerSearch for dyn Container + 'a {
    fn search_focused(&self) -> FocusResult<(&Component, ComponentPos)> {
        let offsets = get_positions(self);
        for (i, child) in self.get_children().iter().enumerate() {
            return match child {
                ContainerChild::Component(child) => match child.get_focus() {
                    Focus::Focus => FocusResult::Focus((child, offsets[i].clone())),
                    Focus::PartialFocus => FocusResult::PartialFocus((child, offsets[i].clone())),
                    Focus::None => continue,
                },
                ContainerChild::Container(child) => match child.search_focused() {
                    FocusResult::Focus((child, pos)) => {
                        FocusResult::Focus((child, offsets[i].clone() + pos))
                    }
                    FocusResult::PartialFocus((child, pos)) => {
                        FocusResult::PartialFocus((child, offsets[i].clone() + pos))
                    }
                    FocusResult::None => continue,
                },
            };
        }
        FocusResult::None
    }

    fn search_focused_mut(&mut self) -> FocusResult<(&mut Component, ComponentPos)> {
        let offsets = get_positions(self);
        for (i, child) in self.get_children_mut().iter_mut().enumerate() {
            return match child {
                ContainerChild::Component(child) => match child.get_focus() {
                    Focus::Focus => FocusResult::Focus((child, offsets[i].clone())),
                    Focus::PartialFocus => FocusResult::PartialFocus((child, offsets[i].clone())),
                    Focus::None => continue,
                },
                ContainerChild::Container(child) => match child.search_focused_mut() {
                    FocusResult::Focus((child, pos)) => {
                        FocusResult::Focus((child, offsets[i].clone() + pos))
                    }
                    FocusResult::PartialFocus((child, pos)) => {
                        FocusResult::PartialFocus((child, offsets[i].clone() + pos))
                    }
                    FocusResult::None => continue,
                },
            };
        }
        FocusResult::None
    }

    fn search_position(&self, pos: ComponentPos) -> Option<(&Component, ComponentPos)> {
        let child_offsets = get_positions(self);
        let child_rects = self.get_children_rectangles();
        let pos_rect = Rect::from(pos.clone());
        for (i, child) in self.get_children().iter().enumerate() {
            if !child_rects[i].intersects(pos_rect) {
                continue;
            }
            return match child {
                ContainerChild::Component(child) => Some((child, child_offsets[i].clone())),
                ContainerChild::Container(child) => {
                    let new_pos = (pos - child_offsets[i].clone()).unwrap();
                    if let Some((child, pos)) = child.search_position(new_pos) {
                        Some((child, child_offsets[i].clone() + pos))
                    } else {
                        None
                    }
                }
            };
        }
        None
    }

    fn search_position_mut(&mut self, pos: ComponentPos) -> Option<(&mut Component, ComponentPos)> {
        let child_offsets = get_positions(self);
        let child_rects = self.get_children_rectangles();
        let pos_rect = Rect::from(pos.clone());
        for (i, child) in self.get_children_mut().iter_mut().enumerate() {
            if !child_rects[i].intersects(pos_rect) {
                continue;
            }
            return match child {
                ContainerChild::Component(child) => Some((child, child_offsets[i].clone())),
                ContainerChild::Container(child) => {
                    let new_pos = (pos - child_offsets[i].clone()).unwrap();
                    if let Some((child, pos)) = child.search_position_mut(new_pos) {
                        Some((child, child_offsets[i].clone() + pos))
                    } else {
                        None
                    }
                }
            };
        }
        None
    }

    fn search_name(&self, path: &str) -> Option<(&ContainerChild, ComponentPos)> {
        let (before, after) = if let Some((before, after)) = path.split_once('.') {
            (before, Some(after))
        } else {
            (path, None)
        };
        let child_offsets = get_positions(self);
        for (i, child) in self.get_children().iter().enumerate() {
            if before != child.as_base().get_name() {
                continue;
            }
            return match child {
                child @ ContainerChild::Component(_) => Some((child, child_offsets[i].clone())),
                ContainerChild::Container(child) => {
                    let Some(after) = after else {
                        continue;
                    };
                    let Some((child, pos)) = child.search_name(after) else {
                        continue ;
                    };
                    Some((child, child_offsets[i].clone() + pos))
                }
            };
        }
        None
    }

    fn search_name_mut(&mut self, path: &str) -> Option<(&mut ContainerChild, ComponentPos)> {
        let (before, after) = if let Some((before, after)) = path.split_once('.') {
            (before, Some(after))
        } else {
            (path, None)
        };
        let child_offsets = get_positions(self);
        for (i, child) in self.get_children_mut().iter_mut().enumerate() {
            if before != child.as_base().get_name() {
                continue;
            }
            return match child {
                child @ ContainerChild::Component(_) => Some((child, child_offsets[i].clone())),
                ContainerChild::Container(child) => {
                    let Some(after) = after else {
                        continue;
                    };
                    let Some((child, pos)) = child.search_name_mut(after) else {
                        continue ;
                    };
                    Some((child, child_offsets[i].clone() + pos))
                }
            };
        }
        None
    }
}
