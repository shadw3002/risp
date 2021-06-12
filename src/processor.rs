use super::{Token, LexerError, Located, ProcessorError, Datum, ToLocated, DatumPair};

use peekmore::{PeekMore, PeekMoreIterator};

type Result<T> = std::result::Result<T, Located<ProcessorError>>;

type TResult = Located<std::result::Result<Token, LexerError>>;

pub struct Processor<TokenIter: Iterator<Item = TResult>> {
    tokens: PeekMoreIterator<TokenIter>,
}

impl<TokenIter: Iterator<Item = TResult>> Iterator for Processor<TokenIter> {
    type Item = Located<std::result::Result<Datum, ProcessorError>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.get_next_datum() {
            Ok(Some(Located{data: datum, location})) => Some(Ok(datum).with_location(location)),
            Ok(None) => None,
            Err(Located{data: e, location}) => Some(Err(e).with_location(location)),
        }
    }
}

impl<TokenIter: Iterator<Item = TResult>> Processor<TokenIter> {
    pub fn from(tokens: TokenIter) -> Processor<TokenIter> {
        Self {
            tokens: tokens.peekmore(),
        }
    }

    fn advance(&mut self) -> Option<TResult> {
        self.tokens.next()
    }

    fn peek(&mut self) -> Option<TResult> {
        let res = self.tokens.peek().map(|t| t.clone());
        self.tokens.advance_cursor();
        res
    }

    fn reset(&mut self) {
        self.tokens.reset_cursor();
    }

    pub fn get_next_datum(&mut self) -> Result<Option<Located<Datum>>> {
        let (token, location) = match self.peek() {
            None => return Ok(None),
            Some(Located{data: Err(e), location}) => return Err(ProcessorError::LexerError(e).with_location(location)),
            Some(Located{data: Ok(token), location}) => (token, location),
        };
        self.reset();

        return Ok(Some(match token {
            Token::Primitive(p) => {self.advance(); Datum::Primitive(p)},
            Token::Identifier(i) => {self.advance(); Datum::Symbol(i)},
            Token::LeftParen => self.get_pair()?,
            Token::RightParen => return located_error!(ProcessorError::UnmatchedParentheses, location),
            Token::VecConsIntro => self.get_vector()?,
            _ => panic!(""),
        }.with_location(location)))
    }

    fn get_vector(&mut self) -> Result<Datum> {
        panic!("TODO")
    }

    fn get_pair(&mut self) -> Result<Datum> {
        let left_paren = self.advance();
        debug_assert_eq!(left_paren.clone().map(|l| l.data), Some(Ok(Token::LeftParen)));
        let pair_location = left_paren.unwrap().location;

        let mut head = Box::new(DatumPair::Empty);
        let mut tail = head.as_mut();

        let mut encounter_period = false;
        return loop {
            match self.peek() {
                Some(Located{data: token, location}) => match
                    token.map_err(|e| ProcessorError::LexerError(e).with_location(location))?
                {
                    Token::Period => {
                        if encounter_period {
                            return located_error!(
                                ProcessorError::UnexpectedToken(Token::Period),
                                location
                            );
                        }
                        
                        self.advance();
                        encounter_period = true;
                        continue;
                    },
                    Token::RightParen => {self.advance(); break Ok(Datum::Pair(head))}, // TODO
                    _ => {
                        self.reset();
                        let Located{data: element, location} = self.get_next_datum()?
                            .ok_or(ProcessorError::UnexpectedEnd.with_location(location))?;
                        
                        match tail {
                            DatumPair::Empty => {
                                head = Box::new(DatumPair::Some(
                                    element.with_location(location), 
                                    Datum::Pair(Box::new(DatumPair::Empty)).with_location(location)
                                ));
                                tail = head.as_mut();
                            },
                            DatumPair::Some(car, cdr) => {
                                if encounter_period {
                                    *cdr = element.with_location(location);
                                    let right_paren = self.advance();
                                    debug_assert_eq!(right_paren.clone().map(|l| l.data), Some(Ok(Token::RightParen)));
                                    break Ok(Datum::Pair(head));
                                }

                                assert_eq!((*cdr).data, Datum::Pair(Box::new(DatumPair::Empty)));

                                let new_tail = 
                                    DatumPair::Some(
                                        element.with_location(location), 
                                        Datum::Pair(Box::new(DatumPair::Empty)).with_location(location)
                                    );
                                *cdr = Datum::Pair(Box::new(new_tail)).with_location(location);
                                tail = if let Located{
                                    data: Datum::Pair(p), 
                                    location
                                } = cdr { p } else { panic!() }
                            }
                        }

                        ()
                    }
                },
                None => return Err(ProcessorError::UnexpectedEnd.with_location(pair_location)),
            }
        };
    }
}

