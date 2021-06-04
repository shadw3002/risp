use super::ToLocated;

#[derive(PartialEq, Debug, Clone)]
pub enum LexerError {
    UnexpectedBegin,
    UnexpectedEnd,
    UnrecognizedToken,
}

impl ToLocated for LexerError {}


#[derive(PartialEq, Debug, Clone)]
pub enum ParserError {

}

impl ToLocated for ParserError {}