use super::ToLocated;

#[derive(PartialEq, Debug, Clone)]
pub enum Token {
    Identifier(String),

    Primitive(Primitive),
    
    LeftParen,        // (
    RightParen,       // )
    VecConsIntro,     // #(
    ByteVecConsIntro, // #u8(
    Quote,            // '
    Quasiquote,       // BackQuote `
    Unquote,          // ,
    UnquoteSplicing,  // ,@
    Period,           // .
}

impl ToLocated for Token {}

#[derive(PartialEq, Debug, Clone)]
pub enum Primitive {
    Boolean(bool),
    Complex(Complex),
    Character(char),
    String(String),
}

#[derive(PartialEq, Debug, Clone)]
pub enum Complex {
    Real(Real),
    Complex(Real, Real),
    Imaginary(Real),
}

#[derive(PartialEq, Debug, Clone)]
pub enum Real {
    PosInf,
    NegInf,
    PosNan,
    NegNan,
    Integer(i64),
    Ration(i64, u64),
    Float(f64),
}

impl Real {
    pub fn reverse(self) -> Self {
        match self {
            Real::PosInf => Real::NegInf,
            Real::NegInf => Real::PosInf,
            Real::PosNan => Real::NegNan,
            Real::NegNan => Real::PosNan,
            Real::Integer(i) => Real::Integer(-i),
            Real::Ration(a, b) => Real::Ration(-a, b),
            Real::Float(f) => Real::Float(-f),
        }
    }
}