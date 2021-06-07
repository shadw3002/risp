use super::{Primitive, ToLocated, GenericPair};

pub type DatumPair = GenericPair<Datum>;

#[derive(PartialEq, Debug, Clone)]
pub enum Datum {
    // simple datum
    Primitive(Primitive),
    Symbol(String),
    ByteVector(),

    // compound datum
    Pair(Box<DatumPair>), // List
    Vector(Vec<Datum>),
    
}

impl ToLocated for Datum {}