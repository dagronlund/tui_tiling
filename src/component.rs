pub mod simple;

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, MouseButton, MouseEventKind};
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, StatefulWidget, Widget},
};

use crate::{Border, Focus, ResizeError};

pub trait ComponentWidget {
    // Handles a mouse event, returns true if component needs to be redrawn
    fn handle_mouse(&mut self, x: u16, y: u16, kind: MouseEventKind) -> bool;
    // Handles a key event, returns true if component needs to be redrawn
    fn handle_key(&mut self, e: KeyEvent) -> bool;
    // Handles a general update, returns true if component needs to be redrawn
    fn handle_update(&mut self) -> bool;
    /// Resizes this component to fit in the new size
    fn resize(&mut self, width: u16, height: u16);
    /// Renders the component to the area specified on the buffer
    fn render(&mut self, area: Rect, buf: &mut Buffer);

    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub trait ComponentBase {
    fn handle_mouse(&mut self, x: u16, y: u16, kind: Option<MouseEventKind>);
    fn handle_key(&mut self, e: KeyEvent) -> Option<Border>;
    fn handle_update(&mut self);

    fn handle_input(&mut self, event: CrosstermEvent) -> Option<Border> {
        match event {
            CrosstermEvent::Key(key) => self.handle_key(key),
            CrosstermEvent::Mouse(event) => {
                self.handle_mouse(event.column, event.row, Some(event.kind));
                None
            }
            CrosstermEvent::Resize(_, _)
            | CrosstermEvent::FocusGained
            | CrosstermEvent::FocusLost
            | CrosstermEvent::Paste(_) => None,
        }
    }

    /// Indicates that all sub-components need to be redrawn
    fn invalidate(&mut self);

    /// Resizes this component to fit in the new size, returning true if this
    /// succeeded, resizing any child components as well, and invalidating this
    /// component and all of its children too
    fn resize(&mut self, width: u16, height: u16) -> Result<(), ResizeError>;

    fn get_width(&self) -> u16;
    fn get_height(&self) -> u16;

    fn is_fixed_width(&self) -> bool;
    fn is_fixed_height(&self) -> bool;

    /// Renders the component to the area specified on the buffer, marking the
    /// component as clean when done
    fn render(&mut self, area: Rect, buf: &mut Buffer);

    /// Returns if any child component have focus or partial focus
    fn get_focus(&self) -> Focus;

    fn get_name(&self) -> String;

    /// Returns which border the x,y position is on, or none if not on a border
    fn get_border(&self, x: u16, y: u16) -> Option<Border>;
}

pub struct Component {
    name: String,
    width: u16,
    height: u16,
    fixed_width: bool,
    fixed_height: bool,
    border_width: u16,
    invalidated: bool,
    focus: Focus,
    widget: Box<dyn ComponentWidget>,
}

impl Component {
    pub fn new(name: String, border_width: u16, widget: Box<dyn ComponentWidget>) -> Self {
        Self {
            name,
            width: 0,
            height: 0,
            fixed_width: false,
            fixed_height: false,
            border_width,
            invalidated: true,
            focus: Focus::None,
            widget,
        }
    }

    pub fn get_border_width(&self) -> u16 {
        self.border_width
    }

    pub fn set_focus(&mut self, focus: Focus) {
        if self.focus != focus {
            self.focus = focus;
            self.invalidate();
        }
    }

    pub fn get_widget(&self) -> &dyn ComponentWidget {
        &*self.widget
    }

    pub fn get_widget_mut(&mut self) -> &mut dyn ComponentWidget {
        &mut *self.widget
    }

    pub fn set_fixed_width(&mut self, fixed_width: Option<u16>) {
        if let Some(width) = fixed_width {
            self.width = width;
            self.fixed_width = true;
        } else {
            self.fixed_width = false;
        }
    }

    pub fn fixed_width(mut self, fixed_width: Option<u16>) -> Self {
        self.set_fixed_width(fixed_width);
        self
    }

    pub fn set_fixed_height(&mut self, fixed_height: Option<u16>) {
        if let Some(height) = fixed_height {
            self.height = height;
            self.fixed_height = true;
        } else {
            self.fixed_height = false;
        }
    }

    pub fn fixed_height(mut self, fixed_height: Option<u16>) -> Self {
        self.set_fixed_height(fixed_height);
        self
    }
}

impl ComponentBase for Component {
    fn handle_mouse(&mut self, x: u16, y: u16, kind: Option<MouseEventKind>) {
        // Check if the mouse event is none, unfocus if true
        let Some(kind) = kind else {
            self.set_focus(Focus::None);
            return;
        };
        // Ignore mouse move events (TODO: Consider reworking this)
        if let MouseEventKind::Moved = kind {
            return;
        }
        // Determine if the mouse event is in the component and the widget
        let in_component = x < self.width && y < self.height;
        let in_widget = x >= self.border_width
            && y >= self.border_width
            && x < self.width - self.border_width
            && y < self.height - self.border_width;
        // Check if the mouse event should focus this component
        match kind {
            MouseEventKind::Down(MouseButton::Left) if in_component => self.set_focus(Focus::Focus),
            MouseEventKind::Drag(MouseButton::Left) if in_component => self.set_focus(Focus::Focus),
            _ => {}
        }
        // Send mouse event to widget if mouse is in widget and component focused
        if in_widget && self.focus == Focus::Focus {
            let (x, y) = (x - self.border_width, y - self.border_width);
            if self.widget.handle_mouse(x, y, kind) {
                self.invalidate();
            }
        }
    }

    fn handle_key(&mut self, e: KeyEvent) -> Option<Border> {
        match self.get_focus() {
            Focus::Focus => match e.code {
                KeyCode::Esc => self.set_focus(Focus::PartialFocus),
                _ => {
                    if self.widget.handle_key(e) {
                        self.invalidate();
                    }
                }
            },
            Focus::PartialFocus => match e.code {
                KeyCode::Up => {
                    self.set_focus(Focus::None);
                    return Some(Border::Top);
                }
                KeyCode::Down => {
                    self.set_focus(Focus::None);
                    return Some(Border::Bottom);
                }
                KeyCode::Left => {
                    self.set_focus(Focus::None);
                    return Some(Border::Left);
                }
                KeyCode::Right => {
                    self.set_focus(Focus::None);
                    return Some(Border::Right);
                }
                KeyCode::Enter => self.set_focus(Focus::Focus),
                _ => {}
            },
            Focus::None if e.code == KeyCode::Enter => self.set_focus(Focus::Focus),
            _ => {}
        }
        None
    }

    fn handle_update(&mut self) {
        if self.widget.handle_update() {
            self.invalidate();
        }
    }

    fn invalidate(&mut self) {
        self.invalidated = true;
    }

    fn resize(&mut self, width: u16, height: u16) -> Result<(), ResizeError> {
        let min = std::cmp::max(self.get_border_width() * 2, 1);
        if width < min || height < min {
            return Err(ResizeError {
                name: self.name.clone(),
                width,
                height,
                border_width: self.border_width,
            });
        }
        if self.width != width || self.height != height {
            self.invalidate();
        }
        if !self.fixed_width {
            self.width = width;
        }
        if !self.fixed_height {
            self.height = height;
        }
        self.widget.resize(width, height);
        Ok(())
    }

    fn get_width(&self) -> u16 {
        self.width
    }

    fn get_height(&self) -> u16 {
        self.height
    }

    fn is_fixed_width(&self) -> bool {
        self.fixed_width
    }

    fn is_fixed_height(&self) -> bool {
        self.fixed_height
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if !self.invalidated {
            return;
        }
        self.invalidated = false;
        // Render borders if they are present
        if self.border_width > 0 {
            let border_color = match self.focus {
                Focus::Focus => Color::Green,
                Focus::PartialFocus => Color::Yellow,
                Focus::None => Color::White,
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(border_color))
                .border_type(BorderType::Rounded);
            if !self.get_name().is_empty() {
                block.title(self.get_name()).render(area, buf)
            } else {
                block.render(area, buf)
            }
        }
        // Do not render widget if borders take whole area
        if area.width <= self.get_border_width() * 2 || area.height <= self.get_border_width() * 2 {
            return;
        }
        // Adjust for borders in determing area to draw
        let rect = Rect {
            x: area.x + self.get_border_width(),
            y: area.y + self.get_border_width(),
            width: area.width - self.get_border_width() * 2,
            height: area.height - self.get_border_width() * 2,
        };
        // Clear widget
        for x in rect.x..(rect.x + rect.width) {
            for y in rect.y..(rect.y + rect.height) {
                buf.get_mut(x, y).reset();
            }
        }
        // Render widget
        self.widget.render(rect, buf);
    }

    fn get_focus(&self) -> Focus {
        self.focus.clone()
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_border(&self, x: u16, y: u16) -> Option<Border> {
        if x >= self.get_width() || y >= self.get_height() {
            return None;
        }
        if x < self.get_border_width() {
            Some(Border::Left)
        } else if x >= self.get_width() - self.get_border_width() {
            Some(Border::Right)
        } else if y < self.get_border_width() {
            Some(Border::Top)
        } else if y >= self.get_height() - self.get_border_width() {
            Some(Border::Bottom)
        } else {
            None
        }
    }
}

pub struct ComponentBaseWidget<'a> {
    component_base: &'a mut dyn ComponentBase,
}

impl<'a> From<&'a mut dyn ComponentBase> for ComponentBaseWidget<'a> {
    fn from(component_base: &'a mut dyn ComponentBase) -> Self {
        Self { component_base }
    }
}

impl<'a> StatefulWidget for ComponentBaseWidget<'a> {
    type State = ();

    fn render(self, area: Rect, buf: &mut Buffer, _: &mut Self::State) {
        self.component_base.render(area, buf);
    }
}
