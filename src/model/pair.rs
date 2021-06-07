


// 6.4 Pairs and lists

#[derive(Debug, Clone, PartialEq)]
pub enum GenericPair<T> {
    Some(T, T),
    Empty,
}

impl<T> Default for GenericPair<T> {
    fn default() -> Self {
        GenericPair::Empty
    }
}

