use std::ops::{Deref, DerefMut};

#[derive(Debug, Copy, Clone)]
pub struct Located<T> {
    pub data: T,
    pub location: Location,
}

#[derive(Debug, Copy, Clone)]
pub struct Location {
    pub row: u32,
    pub col: u32,
}

pub trait ToLocated {
    fn with_location(self, location: Location) -> Located<Self>
    where
        Self: Sized,
    {
        Located::<Self> {
            data: self,
            location: location,
        }
    }
}

impl<T, E> ToLocated for std::result::Result<T, E> {}

impl<T> Located<T> {
    pub fn extract(self) -> T {
        self.data
    }
}

impl<T> Deref for Located<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Located<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}