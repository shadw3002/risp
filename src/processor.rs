use super::{Token, LexerError, Located, ProcessorError, Datum, ToLocated, DatumPair, Primitive, Complex, Real};

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

    fn peek_without_location(&mut self) -> Option<std::result::Result<Token, LexerError>> {
        self.peek().map(|l| l.data)
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
            // simple datum
            Token::Primitive(p) => {self.advance(); Datum::Primitive(p)},
            Token::ByteVecConsIntro => self.get_bytevector()?.data,
            Token::Identifier(i) => {self.advance(); Datum::Symbol(i)},

            // compound datum
            Token::LeftParen => self.get_pair()?.data,
            Token::VecConsIntro => self.get_vector()?.data,
            
            // abbreviation
            Token::Quote => self.get_transform(token, "quote")?.data,
            Token::Quasiquote => self.get_transform(token, "quasiquote")?.data,
            Token::Unquote => self.get_transform(token, "unquote")?.data,
            Token::UnquoteSplicing => self.get_transform(token, "unquote-splicing")?.data,

            Token::RightParen => return located_error!(ProcessorError::UnmatchedParentheses, location),
            _ => return located_error!(ProcessorError::UnexpectedToken(token), location),
        }.with_location(location)))
    }

    fn get_transform(&mut self, token: Token, symbol: &str) -> Result<Located<Datum>> {
        let start = self.advance();
        debug_assert_eq!(start.clone().map(|l| l.data), Some(Ok(token)));
        let start_location = start.unwrap().location;

        let inner = self.get_next_datum()?;
        match inner {
            None => located_error!(ProcessorError::UnexpectedEnd, start_location),
            Some(datum) => Ok(Datum::Pair(Box::new(DatumPair::Some(
                Datum::Symbol(symbol.to_string()).with_location(start_location),
                Datum::Pair(Box::new(DatumPair::Some(
                    datum.clone(),
                    Datum::Pair(Box::new(DatumPair::Empty)).with_location(datum.location),
                ))).with_location(datum.location)
            ))).with_location(start_location)),
        }
    }

    fn get_bytevector(&mut self) -> Result<Located<Datum>> {
        let leftveccon = self.advance();
        debug_assert_eq!(leftveccon.clone().map(|l| l.data), Some(Ok(Token::VecConsIntro)));
        let pair_location = leftveccon.unwrap().location;

        let mut bytes = vec![];
        return Ok(loop {
            match self.advance() {
                None => return located_error!(ProcessorError::UnexpectedEnd, panic!("TODO")),
                Some(Located{data, location}) => match 
                    data.map_err(|e| ProcessorError::LexerError(e).with_location(location))? 
                {
                    Token::RightParen => break Datum::ByteVector(bytes),
                    Token::Primitive(Primitive::Complex(Complex::Real(Real::Integer(i)))) => {
                        match i {
                            0..=255 => bytes.push(i as u8),
                            i => return located_error!(
                                ProcessorError::UnexpectedToken(Token::Primitive(Primitive::Complex(Complex::Real(Real::Integer(i))))),
                                location
                            ),
                        }
                    },
                    token => return located_error!(ProcessorError::UnexpectedToken(token), location),
                },
            }
        }.with_location(pair_location))
    }

    fn get_vector(&mut self) -> Result<Located<Datum>> {
        let leftveccon = self.advance();
        debug_assert_eq!(leftveccon.clone().map(|l| l.data), Some(Ok(Token::VecConsIntro)));
        let pair_location = leftveccon.unwrap().location;

        let mut datums = vec![];
        while self.peek_without_location() != Some(Ok(Token::RightParen)) {
            self.reset();
            match self.get_next_datum()? {
                None => return located_error!(ProcessorError::UnexpectedEnd, panic!("TODO")),
                Some(datum) => datums.push(datum),
            }
        }

        Ok(Datum::Vector(datums).with_location(pair_location))
    }

    fn get_pair(&mut self) -> Result<Located<Datum>> {
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
                    Token::RightParen => {self.advance(); break Ok(Datum::Pair(head).with_location(pair_location))}, // TODO
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
                                    break Ok(Datum::Pair(head).with_location(pair_location));
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

