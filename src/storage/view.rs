use std::ops::Deref;
use crate::component::Component;

pub struct View<'a, T: Component> {
    pub data: &'a T
}

impl<'a, T: Component> View<'a, T> {
    pub fn new(data: &'a T) -> View<'a, T> {
        Self { data }
    }
}

impl<'a, T: Component> Deref for View<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

pub struct ViewMut<'a, T: Component> {
    pub data: & 'a mut T
}