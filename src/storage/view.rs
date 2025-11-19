use crate::component::Component;

pub struct View<'a, T: Component> {
    pub data: & 'a [T]
}

pub struct ViewMut<'a, T: Component> {
    pub data: & 'a mut [T]
}