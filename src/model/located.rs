use std::ops::{Deref, DerefMut};

#[derive(Debug, Copy, Clone)]
pub struct Located<T> {
    pub data: T,
    pub location: Option<Location>,
}

#[derive(Debug, Copy, Clone)]
pub struct Location {
    pub row: u32,
    pub col: u32,
}

pub trait ToLocated {
    fn with_locate(self, location: Location) -> Located<Self>
    where
        Self: Sized,
    {
        Located::<Self> {
            data: self,
            location: Some(location),
        }
    }

    fn without_locate(self) -> Located<Self>
    where
        Self: Sized,
    {
        Located::<Self> {
            data: self,
            location: None,
        }
    }
}

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