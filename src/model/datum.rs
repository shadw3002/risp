use super::{Primitive, ToLocated, GenericPair, Located};

pub type DatumPair = GenericPair<Located<Datum>>;

#[derive(PartialEq, Debug, Clone)]
pub enum Datum {
    // simple datum
    Primitive(Primitive),
    Symbol(String),
    ByteVector(),

    // compound datum
    Pair(Box<DatumPair>), // List
    Vector(Vec<Located<Datum>>),
    
}

impl ToLocated for Datum {}