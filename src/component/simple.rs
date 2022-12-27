use crossterm::event::{KeyEvent, MouseEventKind};
use tui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Paragraph, Widget},
};

use crate::component::ComponentWidget;

pub struct ComponentWidgetSimple {
    text: String,
    style: Style,
    alignment: Alignment,
}

impl ComponentWidgetSimple {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            style: Style::default(),
            alignment: Alignment::Left,
        }
    }

    pub fn get_text(&self) -> String {
        self.text.clone()
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    pub fn text(mut self, text: String) -> Self {
        self.text = text;
        self
    }

    pub fn get_style(&self) -> Style {
        self.style.clone()
    }

    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn get_alignment(&self) -> Alignment {
        self.alignment.clone()
    }

    pub fn set_alignment(&mut self, alignment: Alignment) {
        self.alignment = alignment;
    }

    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }
}

impl ComponentWidget for ComponentWidgetSimple {
    fn handle_mouse(&mut self, _x: u16, _y: u16, _e: MouseEventKind) {}

    fn handle_key(&mut self, _e: KeyEvent) {}

    fn resize(&mut self, _width: u16, _height: u16) {}

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.get_text())
            .style(self.get_style())
            .alignment(self.get_alignment())
            .render(area, buf)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
