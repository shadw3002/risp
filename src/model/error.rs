use super::{ToLocated, Token};

macro_rules! error {
    ($arg:expr) => {
        Err($arg);
    };
}

macro_rules! located_error {
    ($arg:expr, $loc:expr) => {
        Err($arg.with_location($loc));
    };
}

#[derive(PartialEq, Debug, Clone)]
pub enum LexerError {
    UnexpectedBegin,
    UnexpectedEnd,
    UnrecognizedToken,
}

impl ToLocated for LexerError {}


#[derive(PartialEq, Debug, Clone)]
pub enum ProcessorError {
    LexerError(LexerError),
    UnmatchedParentheses,
    UnexpectedEnd,
    UnexpectedToken(Token),
}

impl ToLocated for ProcessorError {}

