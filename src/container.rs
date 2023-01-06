pub mod list;
pub mod search;

use tui::layout::Rect;

use crate::component::{Component, ComponentBase};

pub enum ContainerChild {
    Container(Box<dyn Container>),
    Component(Component),
}

impl ContainerChild {
    pub fn as_base(&self) -> &dyn ComponentBase {
        match self {
            Self::Container(container) => container.as_base(),
            Self::Component(component) => component,
        }
    }

    pub fn as_base_mut(&mut self) -> &mut dyn ComponentBase {
        match self {
            Self::Container(container) => container.as_base_mut(),
            Self::Component(component) => component,
        }
    }

    pub fn unwrap_component(&self) -> &Component {
        match self {
            Self::Component(component) => component,
            Self::Container(_) => panic!("Found container, not component!"),
        }
    }

    pub fn unwrap_component_mut(&mut self) -> &mut Component {
        match self {
            Self::Component(component) => component,
            Self::Container(_) => panic!("Found container, not component!"),
        }
    }
}

impl From<Box<dyn Container>> for ContainerChild {
    fn from(b: Box<dyn Container>) -> Self {
        Self::Container(b)
    }
}

impl<T> From<T> for ContainerChild
where
    T: Container + 'static,
{
    fn from(b: T) -> Self {
        Self::Container(Box::new(b))
    }
}

impl From<Component> for ContainerChild {
    fn from(c: Component) -> Self {
        Self::Component(c)
    }
}

pub trait Container
where
    Self: ComponentBase,
{
    fn get_children(&self) -> &Vec<ContainerChild>;
    fn get_children_mut(&mut self) -> &mut Vec<ContainerChild>;
    fn get_children_rectangles(&self) -> Vec<Rect>;

    fn as_base(&self) -> &dyn ComponentBase;
    fn as_base_mut(&mut self) -> &mut dyn ComponentBase;

    fn is_resizable(&self) -> bool;

    fn as_container(&self) -> &dyn Container;
    fn as_container_mut(&mut self) -> &mut dyn Container;
}
