use std::cmp::Ordering;

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEventKind};
use tui::{
    buffer::Buffer,
    layout::{Direction, Rect},
};

use crate::{
    container::search::ContainerSearch, container::*, pos::*, Border, Focus, FocusResult,
    ResizeError,
};

fn find_next_pos(
    pos: ComponentPos,
    border: Border,
    component_width: u16,
    component_height: u16,
    container_width: u16,
    container_height: u16,
) -> Option<ComponentPos> {
    match border {
        Border::Top if pos.y > 0 => Some(ComponentPos {
            x: pos.x,
            y: pos.y - 1,
        }),
        Border::Bottom if pos.y + component_height < container_height => Some(ComponentPos {
            x: pos.x,
            y: pos.y + component_height,
        }),
        Border::Left if pos.x > 0 => Some(ComponentPos {
            x: pos.x - 1,
            y: pos.y,
        }),
        Border::Right if pos.x + component_width < container_width => Some(ComponentPos {
            x: pos.x + component_width,
            y: pos.y,
        }),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Resize {
    LeftTop {
        mouse_offset: u16,
        child_index: usize,
    },
    RightBottom {
        mouse_offset: u16,
        child_index: usize,
    },
    None,
}

impl Resize {
    fn new(mouse_offset: u16, border: Border, child_index: usize, child_len: usize) -> Self {
        let first = child_index == 0;
        let last = child_index == child_len - 1;
        match border {
            Border::Left | Border::Top if !first => Self::LeftTop {
                mouse_offset,
                child_index,
            },
            Border::Right | Border::Bottom if !last => Self::RightBottom {
                mouse_offset,
                child_index,
            },
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum SizingConstraint {
    Fixed(u16),
    Ratio(f64),
}

pub struct ContainerList {
    name: String,
    orientation: Direction,
    resizable: bool,
    resize: Resize,
    width: u16,
    height: u16,
    children: Vec<ContainerChild>,
}

impl ContainerList {
    pub fn new(
        name: String,
        orientation: Direction,
        resizable: bool,
        width: u16,
        height: u16,
    ) -> Self {
        Self {
            name,
            orientation,
            resizable,
            resize: Resize::None,
            width,
            height,
            children: Vec::new(),
        }
    }

    /// Gets the ratios of all the component sizes to the total
    fn get_sizing_constraints(&self) -> Option<Vec<SizingConstraint>> {
        // Collects initial fixed constraints and current ratio sizing
        let constraints = self
            .children
            .iter()
            .map(|c| c.as_base())
            .map(|c| match self.orientation {
                Direction::Horizontal if c.is_fixed_width() => {
                    SizingConstraint::Fixed(c.get_width())
                }
                Direction::Horizontal => SizingConstraint::Ratio(c.get_width() as f64),
                Direction::Vertical if c.is_fixed_height() => {
                    SizingConstraint::Fixed(c.get_height())
                }
                Direction::Vertical => SizingConstraint::Ratio(c.get_height() as f64),
            })
            .collect::<Vec<SizingConstraint>>();
        // Calculates the total size/count of all ratio controlled components
        let total_ratio = constraints
            .iter()
            .map(|c| match c {
                SizingConstraint::Fixed(_) => 0.0,
                SizingConstraint::Ratio(r) => *r,
            })
            .sum::<f64>();
        let num_ratio = constraints
            .iter()
            .map(|c| match c {
                SizingConstraint::Fixed(_) => 0,
                SizingConstraint::Ratio(_) => 1,
            })
            .sum::<u16>();
        // If any ratio-controlled size is zero, fallback on assigning same
        // ratio to all ratio-controlled components
        let any_ratio_zero = constraints
            .iter()
            .any(|c| matches!(c, SizingConstraint::Ratio(r) if *r == 0.0));

        // Calculate normalized ratios
        let constraints = constraints
            .iter()
            .map(|c| match c {
                SizingConstraint::Fixed(f) => SizingConstraint::Fixed(*f),
                SizingConstraint::Ratio(_) if any_ratio_zero => {
                    SizingConstraint::Ratio(1.0 / num_ratio as f64)
                }
                SizingConstraint::Ratio(r) => SizingConstraint::Ratio(r / total_ratio),
            })
            .collect::<Vec<SizingConstraint>>();
        Some(constraints)
    }

    /// Sets all the children to be proportioned sizes in the container
    fn calculate_sizes(constraints: Vec<SizingConstraint>, size: u16) -> Option<Vec<u16>> {
        // Splits the constraints into fixed and ratio lists
        let fixed_constraints = constraints
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match c {
                SizingConstraint::Fixed(f) => Some((i, *f)),
                _ => None,
            })
            .collect::<Vec<(usize, u16)>>();
        let ratio_constraints = constraints
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match c {
                SizingConstraint::Ratio(r) => Some((i, *r)),
                _ => None,
            })
            .collect::<Vec<(usize, f64)>>();

        let mut result = vec![0].repeat(constraints.len());
        let mut result_total = 0;
        // Assign and sum fixed constraints first
        for (i, f) in fixed_constraints {
            result[i] = f;
            result_total += result[i];
            // No room for fixed components
            if result_total > size {
                return None;
            }
        }
        let size = size - result_total;
        let mut result_total = 0;

        // Assign ratio constraints next, checking if no space
        let num_ratio_constraints = ratio_constraints.len();
        for (index, (i, r)) in ratio_constraints.into_iter().enumerate() {
            if index < num_ratio_constraints - 1 {
                result[i] = (r * size as f64) as u16;
            } else {
                result[i] = size - result_total;
            }
            result_total += result[i];
            // Ratio results in too small of a component
            if result[i] == 0 {
                return None;
            }
        }
        Some(result)
    }

    /// Adds a new component to the container, resizes the existing children
    /// to fit the new component, and returns false if there was no room
    /// available for the component
    pub fn add_component(&mut self, child: Component) -> Result<(), ResizeError> {
        self.children.push(ContainerChild::Component(child));
        self.resize(self.get_width(), self.get_height())
    }

    /// Adds a new container to the container, resizes the existing children
    /// to fit the new container, and returns false if there was no room
    /// available for the container
    pub fn add_container(&mut self, child: Box<dyn Container>) -> Result<(), ResizeError> {
        self.children.push(ContainerChild::Container(child));
        self.resize(self.get_width(), self.get_height())
    }

    pub fn get_orientation(&self) -> Direction {
        self.orientation.clone()
    }

    fn handle_resize(
        &mut self,
        mouse_offset_next: u16,
        orientation: Direction,
        kind: MouseEventKind,
    ) {
        // Clear resizing if not resizable or mouse event is not left drag
        if !self.resizable || kind != MouseEventKind::Drag(MouseButton::Left) {
            self.resize = Resize::None;
            return;
        }
        // Get indices of affected child components and the drag delta
        let (index0, index1, delta) = match self.resize {
            Resize::LeftTop {
                mouse_offset,
                child_index,
            } => (
                child_index - 1,
                child_index,
                mouse_offset_next as i16 - mouse_offset as i16,
            ),
            Resize::RightBottom {
                mouse_offset,
                child_index,
            } => (
                child_index,
                child_index + 1,
                mouse_offset_next as i16 - mouse_offset as i16,
            ),
            Resize::None => return,
        };
        // Make sure that this will not resize a fixed size component on either side
        match orientation {
            Direction::Horizontal
                if (self.get_children()[index0].as_base().is_fixed_width()
                    || self.get_children()[index1].as_base().is_fixed_width()) =>
            {
                return
            }
            Direction::Vertical
                if (self.get_children()[index0].as_base().is_fixed_height()
                    || self.get_children()[index1].as_base().is_fixed_height()) =>
            {
                return
            }
            _ => {}
        }
        // Get current sizes of child components
        let (width0, height0, width1, height1) = (
            self.get_children()[index0].as_base().get_width(),
            self.get_children()[index0].as_base().get_height(),
            self.get_children()[index1].as_base().get_width(),
            self.get_children()[index1].as_base().get_height(),
        );
        // Use orientation and drag to calculate new width/height
        let (width0, height0, width1, height1) = match orientation {
            Direction::Horizontal => {
                let width0 = width0 as i16 + delta;
                let width1 = width1 as i16 - delta;
                if width0 <= 0 || width1 <= 0 {
                    self.resize = Resize::None;
                    return;
                }
                (width0 as u16, height0, width1 as u16, height1)
            }
            Direction::Vertical => {
                let height0 = height0 as i16 + delta;
                let height1 = height1 as i16 - delta;
                if height0 <= 0 || height1 <= 0 {
                    self.resize = Resize::None;
                    return;
                }
                (width0, height0 as u16, width1, height1 as u16)
            }
        };
        // Use delta to determine which component to size first, none if 0
        let (index0, width0, height0, index1, width1, height1) = match delta.cmp(&0) {
            Ordering::Less => (index0, width0, height0, index1, width1, height1),
            Ordering::Greater => (index1, width1, height1, index0, width0, height0),
            Ordering::Equal => return,
        };
        // Resize component that is getting smaller first, there should be
        // no issue resizing component that is getting bigger
        if self.get_children_mut()[index0]
            .as_base_mut()
            .resize(width0, height0)
            .is_err()
        {
            self.resize = Resize::None;
            return;
        }
        assert_eq!(
            self.get_children_mut()[index1]
                .as_base_mut()
                .resize(width1, height1),
            Ok(())
        );
        // Update resize event to current offset
        self.resize = match self.resize {
            Resize::LeftTop {
                mouse_offset: _,
                child_index,
            } => Resize::LeftTop {
                mouse_offset: mouse_offset_next,
                child_index,
            },
            Resize::RightBottom {
                mouse_offset: _,
                child_index,
            } => Resize::RightBottom {
                mouse_offset: mouse_offset_next,
                child_index,
            },
            Resize::None => Resize::None,
        };
    }
}

impl Container for ContainerList {
    fn get_children(&self) -> &Vec<ContainerChild> {
        &self.children
    }

    fn get_children_mut(&mut self) -> &mut Vec<ContainerChild> {
        &mut self.children
    }

    fn get_children_rectangles(&self) -> Vec<Rect> {
        let mut pos = ComponentPos::default();
        let mut rects = Vec::new();
        for child in &self.children {
            rects.push(Rect {
                x: pos.x,
                y: pos.y,
                width: child.as_base().get_width(),
                height: child.as_base().get_height(),
            });
            match self.orientation {
                Direction::Horizontal => pos.x += child.as_base().get_width(),
                Direction::Vertical => pos.y += child.as_base().get_height(),
            }
        }
        rects
    }

    fn as_base(&self) -> &dyn ComponentBase {
        self
    }

    fn as_base_mut(&mut self) -> &mut dyn ComponentBase {
        self
    }

    fn is_resizable(&self) -> bool {
        self.resizable
    }

    fn as_container(&self) -> &dyn Container {
        self
    }

    fn as_container_mut(&mut self) -> &mut dyn Container {
        self
    }
}

impl ComponentBase for ContainerList {
    fn handle_mouse(&mut self, x: u16, y: u16, kind: Option<MouseEventKind>) {
        // Check if the mouse event is none
        let Some(kind) = kind else {
            // Issue none to all children
            for child in &mut self.children {
                child.as_base_mut().handle_mouse(0, 0, None);
            }
            // Clear current resizing
            self.resize = Resize::None;
            return;
        };
        // Ignore mouse move events (TODO: Consider reworking this)
        if let MouseEventKind::Moved = kind {
            return;
        }
        // Handle an ongoing resize event
        let mouse_offset = match self.orientation {
            Direction::Horizontal => x,
            Direction::Vertical => y,
        };
        self.handle_resize(mouse_offset, self.get_orientation(), kind);
        let mouse_pos = ComponentPos { x, y };
        let child_rects = self.as_container().get_children_rectangles();
        // Iterate through children, dispatching mouse event if intersects
        for (i, child) in self.children.iter_mut().enumerate() {
            // Check mouse intersection, issue none if no intersection
            if !mouse_pos.intersects_rect(child_rects[i]) {
                child.as_base_mut().handle_mouse(0, 0, None);
                continue;
            }
            let (child_x, child_y) = (x - child_rects[i].x, y - child_rects[i].y);
            // Check if mouse intersects a child border
            if let Some(border) = child.as_base().get_border(child_x, child_y) {
                if let MouseEventKind::Down(MouseButton::Left) = kind {
                    self.resize = Resize::new(mouse_offset, border, i, child_rects.len());
                }
            }
            // Mouse intersected the child component/container
            child
                .as_base_mut()
                .handle_mouse(child_x, child_y, Some(kind));
        }
    }

    fn handle_key(&mut self, event: KeyEvent) -> Option<Border> {
        let (container_width, container_height) = (self.get_width(), self.get_height());
        // Send key event to the (partially) focused component
        match self.as_container_mut().search_focused_mut() {
            FocusResult::Focus((component, pos)) | FocusResult::PartialFocus((component, pos)) => {
                // Process key and see if focus needs to change
                let Some(border) = component.handle_key(event) else {
                    return None;
                };
                // Look for position of next component to focus
                let next_pos = find_next_pos(
                    pos,
                    border,
                    component.get_width(),
                    component.get_height(),
                    container_width,
                    container_height,
                );
                // Check the position actually exists
                let Some(next_pos) = next_pos else {
                    component.set_focus(Focus::PartialFocus);
                    return None;
                };
                // Partially focus component
                if let Some((component_next, _)) =
                    self.as_container_mut().search_position_mut(next_pos)
                {
                    component_next.set_focus(Focus::PartialFocus);
                }
            }
            FocusResult::None => {
                // If nothing has focus, check if the right keys were pressed
                match event.code {
                    KeyCode::Enter
                    | KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right => {}
                    _ => return None,
                }
                // Find something to focus
                if let Some((component, _)) = self
                    .as_container_mut()
                    .search_position_mut(ComponentPos { x: 0, y: 0 })
                {
                    component.set_focus(Focus::PartialFocus);
                }
            }
        }
        None
    }

    fn invalidate(&mut self) {
        for component in &mut self.children {
            component.as_base_mut().invalidate();
        }
    }

    fn resize(&mut self, width: u16, height: u16) -> Result<(), ResizeError> {
        if self.width == width && self.height == height {
            return Ok(());
        }
        // Get constraints for resizing later
        let Some(constraints) = self.get_sizing_constraints() else {
            return Err(ResizeError {
                name: self.get_name(),
                width,
                height,
                border_width: 0
            });
        };
        // Get current sizing
        let old_dimensions = self
            .children
            .iter()
            .map(|c| (c.as_base().get_width(), c.as_base().get_height()))
            .collect::<Vec<(u16, u16)>>();
        // Calculate new sizing
        let Some(new_sizes) = ContainerList::calculate_sizes(
            constraints,
            match self.orientation {
                Direction::Horizontal => width,
                Direction::Vertical => height,
            },
        ) else {
            return Err(ResizeError {
                name: self.get_name(),
                width,
                height,
                border_width: 0
            });
        };
        let new_sizes = new_sizes
            .iter()
            .map(|s| match self.orientation {
                Direction::Horizontal => (*s, height),
                Direction::Vertical => (width, *s),
            })
            .collect::<Vec<(u16, u16)>>();
        for (i, size) in new_sizes.iter().enumerate().take(self.children.len()) {
            if let Err(err) = self.children[i].as_base_mut().resize(size.0, size.1) {
                for (i, dim) in old_dimensions.iter().enumerate().take(self.children.len()) {
                    let _ = self.children[i].as_base_mut().resize(dim.0, dim.1);
                }
                return Err(err);
            }
        }
        self.width = width;
        self.height = height;
        self.invalidate();
        Ok(())
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        assert_eq!(area.width, self.width);
        assert_eq!(area.height, self.height);
        let child_rects = self.as_container().get_children_rectangles();
        for (i, child) in self.children.iter_mut().enumerate() {
            child.as_base_mut().render(
                Rect {
                    x: child_rects[i].x + area.x,
                    y: child_rects[i].y + area.y,
                    height: child_rects[i].height,
                    width: child_rects[i].width,
                },
                buf,
            );
        }
    }

    fn get_width(&self) -> u16 {
        self.width
    }

    fn get_height(&self) -> u16 {
        self.height
    }

    fn is_fixed_width(&self) -> bool {
        self.children.iter().all(|c| c.as_base().is_fixed_width())
    }

    fn is_fixed_height(&self) -> bool {
        self.children.iter().all(|c| c.as_base().is_fixed_height())
    }

    fn get_focus(&self) -> Focus {
        for component in &self.children {
            match component.as_base().get_focus() {
                Focus::Focus => return Focus::Focus,
                Focus::PartialFocus => return Focus::PartialFocus,
                _ => {}
            }
        }
        Focus::None
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_border(&self, x: u16, y: u16) -> Option<Border> {
        let pos = ComponentPos { x, y };
        let child_rects = self.as_container().get_children_rectangles();
        for (i, component) in self.children.iter().enumerate() {
            let first = i == 0;
            let last = i == self.children.len() - 1;
            // Check if position in this child, otherwise try next one
            if !pos.intersects_rect(child_rects[i]) {
                continue;
            }
            // If this child has no matching border, then no other child will
            let border = component
                .as_base()
                .get_border(x - child_rects[i].x, y - child_rects[i].y);
            let Some(border) = border else {
                return None;
            };
            // Check if there is a matching border
            return match (&self.orientation, border) {
                (Direction::Horizontal, Border::Top) => Some(Border::Top),
                (Direction::Horizontal, Border::Bottom) => Some(Border::Bottom),
                (Direction::Horizontal, Border::Left) if first => Some(Border::Left),
                (Direction::Horizontal, Border::Right) if last => Some(Border::Right),
                (Direction::Vertical, Border::Top) if first => Some(Border::Top),
                (Direction::Vertical, Border::Bottom) if last => Some(Border::Bottom),
                (Direction::Vertical, Border::Left) => Some(Border::Left),
                (Direction::Vertical, Border::Right) => Some(Border::Right),
                _ => None,
            };
        }
        None
    }
}
