
#[derive(Debug)]
pub enum Expression {
    Nil,
    Boolean(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Cons(Box<Expression>, Box<Expression>),
    Symbol(String),
    NoMatch,
}