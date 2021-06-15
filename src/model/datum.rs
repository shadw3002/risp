use super::{Primitive, ToLocated, GenericPair, Located, Real};

pub type DatumPair = GenericPair<Located<Datum>>;

#[derive(PartialEq, Debug, Clone)]
pub enum Datum {
    // simple datum
    Primitive(Primitive),
    Symbol(String),
    ByteVector(Vec<u8>),

    // compound datum
    Pair(Box<DatumPair>), // List
    Vector(Vec<Located<Datum>>),
    
}

impl ToLocated for Datum {}