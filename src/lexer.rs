macro_rules! error {
    ($arg:expr) => {
        Err($arg);
    };
}

macro_rules! located_error {
    ($arg:expr, $loc:expr) => {
        Err($arg.with_locate($loc));
    };
}

use std::iter::Iterator;
use peekmore::{PeekMore, PeekMoreIterator};

use super::{Token, Primitive, Complex, Real, Located, Location, LexerError, ToLocated};

pub struct Lexer<CharIter: Iterator<Item = char>> {
    char_stream: PeekMoreIterator<CharIter>,
    advance_location: Location,
    peek_location: Location,
}

type Result<T> = std::result::Result<T, Located<LexerError>>;

impl<CharIter: Iterator<Item = char>> Iterator for Lexer<CharIter> {
    type Item = Result<Located<Token>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.get_next_token() {
            Ok(None) => None,
            Ok(Some(token)) => Some(Ok(token)),
            Err(e) => Some(Err(e)),
        }
    }
}

impl<CharIter: Iterator<Item = char>> Lexer<CharIter> {
    pub fn new(char_stream: CharIter) -> Self {
        Self {
            char_stream: char_stream.peekmore(),
            advance_location: Location{row: 0, col: 0},
            peek_location: Location{row: 0, col: 0},
        }
    }

    fn get_next_token(&mut self) -> Result<Option<Located<Token>>> {
        while let (Some(ch), location) = self.peek_with_location() {
            return Ok(Some(match ch {
                _ if is_whitespace(ch) => {self.advance(); continue},
                ';'  => {self.reset(); self.skip_line_comment(); continue},
                '('  => {self.advance(); Token::LeftParen},
                ')'  => {self.advance(); Token::RightParen},
                '#'  => match self.peek() {
                    Some('|')  => {self.reset(); self.skip_block_comment(); continue},
                    Some('(')  => {self.advance_n(2); Token::VecConsIntro},
                    Some('t')  => {self.advance_n(2); Token::Primitive(Primitive::Boolean(true))},
                    Some('f')  => {self.advance_n(2); Token::Primitive(Primitive::Boolean(false))},
                    Some('\\') => match self.peek() {
                        Some(ch) => {self.advance_n(3); Token::Primitive(Primitive::Character(ch))},
                        None => return located_error!(LexerError::UnexpectedEnd, location),
                    },
                    Some('u') => match (self.peek(), self.peek()) {
                        (Some('8'), Some('(')) => {self.advance_n(4); Token::ByteVecConsIntro},
                        _ => return located_error!(LexerError::UnrecognizedToken, location),
                    }
                    Some('e') | Some('i') | Some('b') | Some('o') | Some('d') | Some('x') => {
                        let (radix, exactness) = self.get_complex_prefix()?;
                        Token::Primitive(Primitive::Complex(self.get_complex(radix, exactness)?))
                    },
                    Some(_) => return located_error!(LexerError::UnrecognizedToken, location),
                    _ => return located_error!(LexerError::UnexpectedEnd, location),
                },
                '\'' => {self.advance(); Token::Quote},
                '`'  => {self.advance(); Token::Quasiquote},
                ','  => match self.peek() {
                    Some('@') => {self.advance_n(2);            Token::UnquoteSplicing},
                    _         => {self.advance(); self.reset(); Token::Unquote},
                },
                '.' => match self.peek() {
                    Some(ch) if is_delimiter(ch) => {self.advance(); Token::Period},
                    None                              => {self.advance(); Token::Period},
                    Some(ch) if '0' <= ch && ch <= '9' => {
                        self.reset(); 
                        Token::Primitive(Primitive::Complex(self.get_complex(Radix::Decimal, true)?))
                    }
                    Some(_) => {self.reset(); self.get_percular_identifier()?},
                },
                '+' | '-' => match self.peek() {
                    Some('0'..='9') => {
                        self.reset(); 
                        Token::Primitive(Primitive::Complex(self.get_complex(Radix::Decimal, true)?))
                    }, 
                    Some('.') => {
                        self.reset(); 
                        Token::Primitive(Primitive::Complex(self.get_complex(Radix::Decimal, true)?))
                    },
                    Some('i') if ({
                        let ch = self.peek();
                        self.char_stream.move_cursor_back().unwrap();
                        ch.is_some() && is_delimiter(ch.unwrap())
                    }) => {
                        self.reset();
                        Token::Primitive(Primitive::Complex(self.get_complex(Radix::Decimal, true)?))
                    },
                    Some('i') if self.has_specific_string("nf.0") => {
                        self.reset();
                        Token::Primitive(Primitive::Complex(self.get_complex(Radix::Decimal, true)?))
                    },
                    Some('n') if self.has_specific_string("an.0") => {
                        self.reset();
                        Token::Primitive(Primitive::Complex(self.get_complex(Radix::Decimal, true)?))
                    },
                    Some(_) | None => {self.reset(); self.get_percular_identifier()?},
                },
                '0'..='9' => {
                    self.reset(); 
                    Token::Primitive(Primitive::Complex(self.get_complex(Radix::Decimal, true)?))
                },
                '"' => {self.reset(); self.get_string()?},
                '|' => {self.reset(); self.get_quoted_identifier()?},
                ch if is_identifier_initial(ch) => {self.reset(); self.get_normal_identifier()?},
                _ => return located_error!(LexerError::UnexpectedBegin, location),
            }.with_locate(location)))
        }

        Ok(None)
    }

    fn has_specific_string(&mut self, text: &str) -> bool {
        let mut i = 0;
        for rch in text.chars() {
            match self.peek() {
                Some(ch) if ch == rch => i += 1,
                Some(_) => {
                    i += 1;
                    self.char_stream.move_cursor_back_by(i).unwrap();
                    return false;
                },
                None => {
                    self.char_stream.move_cursor_back_by(i).unwrap();
                    return false;
                },
            };
        }
        self.char_stream.move_cursor_back_by(text.len()).unwrap();

        true
    }

    // <num> -> <refix> <complex> 
    fn get_complex_prefix(&mut self) -> Result<(Radix, bool)> {
        let ch = self.advance();
        debug_assert_eq!(Some('#'), ch);

        let mut exactness = None;
        let mut radix = None;
        match self.advance() {
            Some('e') => exactness = Some(true),
            Some('i') => exactness = Some(false),
            Some('b') => radix = Some(2),
            Some('o') => radix = Some(8),
            Some('d') => radix = Some(10),
            Some('x') => radix = Some(16),
            Some(_) => return located_error!(LexerError::UnrecognizedToken, self.advance_location),
            None => return located_error!(LexerError::UnexpectedEnd, self.advance_location),
        }

        if self.peek() == Some('#') {
            self.advance();
            let ch = self.advance();
            match ch {
                Some('e') |
                Some('i') => if exactness.is_some() {
                    return located_error!(LexerError::UnrecognizedToken, self.advance_location)
                },
                Some('b') |
                Some('o') |
                Some('d') |
                Some('x') => if radix.is_some() {
                    return located_error!(LexerError::UnrecognizedToken, self.advance_location)
                },
                Some(_) => return located_error!(LexerError::UnrecognizedToken, self.advance_location),
                None => return located_error!(LexerError::UnexpectedEnd, self.advance_location),
            }
            match ch {
                Some('e') => exactness = Some(true),
                Some('i') => exactness = Some(false),
                Some('b') => radix = Some(2),
                Some('o') => radix = Some(8),
                Some('d') => radix = Some(10),
                Some('x') => radix = Some(16),
                Some(_) => return located_error!(LexerError::UnrecognizedToken, self.advance_location),
                None => return located_error!(LexerError::UnexpectedEnd, self.advance_location),
            }
        } else {
            self.reset();
        }

        if exactness.is_none() {
            exactness = Some(true);
        }
        if radix.is_none() {
            radix = Some(10);
        }

        Ok((Radix::from(radix.unwrap()).unwrap(), exactness.unwrap()))
    }

    fn get_complex(&mut self, radix: Radix, exactness: bool) -> Result<Complex> {
        // case 11 12
        match (self.peek(), self.peek(), self.peek()) {
            (Some('+'), Some('i'), last) |
            (Some('-'), Some('i'), last) 
            if last.is_none() || is_delimiter(last.unwrap())
            => {
                self.reset();
                return Ok(Complex::Imaginary(self.get_single_i()?))
            },
            _ => (),
        }
        self.reset();

        let c1 = self.peek();
        self.reset();

        let r1 = self.get_real(radix, exactness)?;

        let (ch, location) = self.peek_with_location();
        Ok(match ch {
            Some(ch) if is_delimiter(ch) => {
                self.reset();
                Complex::Real(r1)
            },
            None => {
                self.reset();
                Complex::Real(r1)
            },
            // case 2
            Some('@') => {
                self.advance();
                let r2 = self.get_real(radix, exactness)?;
                Complex::Complex(r1, r2)
            },
            // case 3 5 7
            Some('+') => if Some('i') == self.peek() {
                self.reset();
                if let Ok(r2) = self.get_infnan() {
                    Complex::Complex(r1, r2)
                } else {
                    self.advance_n(2);
                    Complex::Complex(r1, Real::Integer(1))
                }
            } else {
                self.reset();
                let r2 = self.get_real(radix, exactness)?;
                let (ch, location) = self.peek_with_location();
                match ch {
                    Some('i') => {
                        self.advance();
                        Complex::Complex(r1, r2)
                    },
                    Some(_) => return located_error!(LexerError::UnrecognizedToken, location),
                    None => return located_error!(LexerError::UnexpectedEnd, location)
                }
            },
            // case 4 6 7
            Some('-') => if Some('i') == self.peek() {
                self.reset();
                if let Ok(r2) = self.get_infnan() {
                    Complex::Complex(r1, r2)
                } else {
                    self.advance_n(2);
                    Complex::Complex(r1, Real::Integer(-1))
                }
            } else {
                self.reset();
                let r2 = self.get_real(radix, exactness)?;
                let (ch, location) = self.peek_with_location();
                match ch {
                    Some('i') => {
                        self.advance();
                        Complex::Complex(r1, r2)
                    },
                    Some(_) => return located_error!(LexerError::UnrecognizedToken, location),
                    None => return located_error!(LexerError::UnexpectedEnd, location)
                }
            },
            // case 8 9 10
            Some('i') => {
                self.advance();
                if c1 != Some('+') && c1 != Some('-') {
                    return located_error!(LexerError::UnrecognizedToken, location);
                }
                Complex::Imaginary(r1)
            },
            Some(_) => return located_error!(LexerError::UnrecognizedToken, location),
        })
    }

    fn get_real(&mut self, radix: Radix, exactness: bool) -> Result<Real> {
        match (self.peek(), self.peek()) {
            (Some('+'), Some('i')) |
            (Some('-'), Some('i')) |
            (Some('+'), Some('n')) |
            (Some('-'), Some('n')) => {
                self.reset();
                self.get_infnan()
            },
            (Some('+'), _) => {
                self.reset();
                self.advance();
                self.get_unreal(radix, exactness)
            },
            (Some('-'), _) => {
                self.reset();
                self.advance();
                self.get_unreal(radix, exactness).map(|r| r.reverse())
            },
            _ => {
                self.reset();
                self.get_unreal(radix, exactness)
            },
        }
    }

    fn get_unreal(&mut self, radix: Radix, _exactness: bool) -> Result<Real> {
        let (ch, location) = self.peek_with_location();
        match ch {
            Some(ch) if radix.contains(ch) => {
                self.reset();
                let n1 = self.get_digit(radix)?;
                assert!(!n1.is_empty());
                match self.peek() {
                    Some(ch) 
                    if is_delimiter(ch) 
                    || ch == '@' || ch == 'i'
                    || ch == '+' || ch == '-' // TODO
                    => {
                        self.reset();
                        let int = n1.parse().unwrap();
                        Ok(Real::Integer(int))
                    },
                    None => {
                        self.reset();
                        let int = n1.parse().unwrap();
                        Ok(Real::Integer(int))
                    },
                    Some('/') => {
                        self.advance();
                        let n2 = self.get_digit(radix)?;
                        assert!(!n2.is_empty());
                        let i1 = n1.parse().unwrap();
                        let i2 = n2.parse().unwrap();
                        Ok(Real::Ration(i1, i2))
                    }
                    Some('.') => {
                        self.advance();
                        let n2 = self.get_digit(radix)?;
                        let suffix = self.get_suffix()?;
                        let f = (n1 + "." + &n2 + &suffix).parse().unwrap();
                        Ok(Real::Float(f))
                    },
                    Some('e') => {
                        self.reset();
                        let suffix = self.get_suffix()?;
                        let f = (n1 + &suffix).parse().unwrap();
                        Ok(Real::Float(f))
                    },
                    Some(_) => return located_error!(LexerError::UnrecognizedToken, location),
                }
            },
            Some('.') => {
                self.advance();
                let n2 = self.get_digit(radix)?;
                assert!(!n2.is_empty());
                let suffix = self.get_suffix()?;
                let unreal = '.'.to_string() + &n2 + &suffix;
                let unreal = unreal.parse().unwrap();
                Ok(Real::Float(unreal))
            },
            Some(_) => return located_error!(LexerError::UnrecognizedToken, location),
            None => return located_error!(LexerError::UnexpectedEnd, location),
        }
    }

    fn get_suffix(&mut self) -> Result<String> {
        let mut res = String::new();
        if self.peek() != Some('e') {
            self.reset();
            Ok(res)
        } else {
            self.advance();
            res.push('e');
            let sign = self.peek();
            if sign == Some('+') || sign == Some('-') {
                res.push(sign.unwrap());
                self.advance();
            } else {
                self.reset();
            }
            let digit = self.get_digit(Radix::Decimal)?;
            assert!(!digit.is_empty());
            Ok(res + &digit)
        }
    }

    fn get_digit(&mut self, radix: Radix) -> Result<String> {
        let mut res = String::new();
        while let Some(ch) = self.peek() {
            if radix.contains(ch) {
                res.push(ch);
                self.advance();
            } else {
                self.reset();
                break;
            }
        }
        Ok(res)
    }

    fn get_infnan(&mut self) -> Result<Real> {
        let sign = self.advance();
        let location = self.advance_location;
        debug_assert!(sign.is_some());
        let sign = sign.unwrap();
        debug_assert!(['+', '-'].contains(&sign));

        if self.has_specific_string("inf.0") {
            self.advance_n(5);
            Ok(if sign == '+' {Real::PosInf} else {Real::NegInf})
        } else if self.has_specific_string("nan.0") {
            self.advance_n(5);
            Ok(if sign == '+' {Real::PosNan} else {Real::NegNan})
        } else {
            located_error!(LexerError::UnrecognizedToken, location)
        }
    }

    fn get_single_i(&mut self) -> Result<Real> {
        let sign = self.advance();
        debug_assert!(sign.is_some());
        let sign = sign.unwrap();
        debug_assert!(['+', '-'].contains(&sign));

        let ch = self.advance();
        debug_assert_eq!(Some('i'), ch);

        Ok(match sign {
            '+' => Real::Integer(1),
            '-' => Real::Integer(-1),
            _   => panic!("unexpect to get here"), 
        })
    }

    fn get_quoted_identifier(&mut self) -> Result<Token> {
        let ch = self.advance();
        debug_assert_eq!(ch, Some('|'));

        let mut identifier_str = String::new();
        while let Some(ch) = self.advance() {
            match ch {
                '|' => return Ok(Token::Identifier(
                    if identifier_str.is_empty() {"||".to_string()} else {identifier_str}
                )),
                ch => identifier_str.push(ch),
            }
        }
        located_error!(LexerError::UnrecognizedToken, self.advance_location)
    }

    fn get_normal_identifier(&mut self) -> Result<Token> {
        let ch = self.advance();
        debug_assert!(ch.is_some());
        debug_assert!(is_identifier_initial(ch.unwrap()));
        let ch = ch.unwrap();

        let mut identifier_string = ch.to_string();
        self.get_subsequent(&mut identifier_string)?;
        Ok(Token::Identifier(identifier_string))
    }

    fn get_subsequent(&mut self, identifier_string: &mut String) -> Result<()> {
        while let (Some(ch), location) = self.peek_with_location() {
            match ch {
                _ if is_identifier_initial(ch) => identifier_string.push(ch),
                '0'..='9' | '+' | '-' | '.' | '@' => identifier_string.push(ch),
                _ if is_delimiter(ch) => {self.reset(); break},
                _ => return located_error!(LexerError::UnrecognizedToken, location),
            }
            self.advance();
        }

        Ok(())
    }

    fn get_string(&mut self) -> Result<Token> {
        let ch = self.advance();
        debug_assert_eq!(ch, Some('\"'));

        let mut string_literal = String::new();
        while let Some(ch) = self.advance() {
            match ch {
                '"' => return Ok(Token::Primitive(Primitive::String(string_literal))),
                '\\' => match self.advance() {
                    Some(ec) => match ec {
                        'a' => string_literal.push('\u{007}'),
                        'b' => string_literal.push('\u{008}'),
                        't' => string_literal.push('\u{009}'),
                        'n' => string_literal.push('\n'),
                        'r' => string_literal.push('\r'),
                        '"' => string_literal.push('"'),
                        '\\' => string_literal.push('\\'),
                        '|' => string_literal.push('|'),
                        'x' => (), // TODO: 'x' for hex value
                        ' ' => (), // TODO: space for nothing
                        _ => return located_error!(LexerError::UnrecognizedToken, self.advance_location),
                    }
                    None => return located_error!(LexerError::UnexpectedEnd, self.advance_location),
                },
                _ => string_literal.push(ch),
            }
        }

        return located_error!(LexerError::UnexpectedEnd, self.advance_location)
    }

    fn get_percular_identifier(&mut self) -> Result<Token> {
        let ch1 = self.advance();
        debug_assert!(ch1.is_some());
        debug_assert!(['+', '-', '.'].contains(&ch1.unwrap()));
        let ch1 = ch1.unwrap();

        let mut identifier_string = ch1.to_string();
        match ch1 {
            '+' | '-' => match self.peek() {
                Some(nc) if is_delimiter(nc) => self.reset(),
                Some(nc) if is_sign_subsequent(nc) => {
                    identifier_string.push(nc);
                    self.advance();
                    self.get_subsequent(&mut identifier_string)?
                },
                Some('.') => {
                    identifier_string.push('.');
                    self.advance();
                    match self.peek().unwrap() {
                        ch if is_sign_subsequent(ch) || ch == '.' => {
                            identifier_string.push(ch);
                            self.advance();
                            self.get_subsequent(&mut identifier_string)?;
                        },
                        _ => return located_error!(LexerError::UnrecognizedToken, self.peek_location),
                    };
                },

                Some(_) => return located_error!(LexerError::UnrecognizedToken, self.advance_location),
                None => return located_error!(LexerError::UnexpectedEnd, self.advance_location),
            },
            '.' => match self.peek().unwrap() {
                ch if is_sign_subsequent(ch) || ch == '.' => {
                    identifier_string.push(ch);
                    self.advance();
                    self.get_subsequent(&mut identifier_string)?;
                },
                _ => return located_error!(LexerError::UnrecognizedToken, self.advance_location),
            },
            _ => panic!("unexpected"),
        }
        Ok(Token::Identifier(identifier_string))
    }

    // fn get_complex_suffix(&mut self, number_literal: &mut String) {
    //     self.advance();
    //     number_literal.push('e');
    //     if let Some(sign) = self.peek() {
    //         if (sign == '+') || (sign == '-') {
    //             number_literal.push(sign);
    //             self.advance();
    //         }
    //     }
    //     self.get_digital(number_literal)
    // }

    fn peek_with_location(&mut self) -> (Option<char>, Location) {
        let location = self.peek_location;
        let ch = self.char_stream.peek().map(|c| *c);
        self.char_stream.advance_cursor();
        if ch.is_some() {
            move_location(ch.unwrap(), &mut self.peek_location)
        }
        (ch, location)
    }

    fn peek(&mut self) -> Option<char> {
        self.peek_with_location().0
    }

    fn reset(&mut self) {
        self.char_stream.reset_cursor();
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.char_stream.next();
        
        if ch.is_some() {
            move_location(ch.unwrap(), &mut self.advance_location)
        }
        self.peek_location = self.advance_location;
        ch
    }

    fn advance_n(&mut self, n: usize) {
        for _ in 0..n {
            self.advance();
        }
    }



    fn skip_line_comment(&mut self) {
        let start = self.advance();
        debug_assert_eq!(Some(';'), start);

        while let Some(ch) = self.advance() {
            match ch {
                '\n' | '\r'  => break,
                _ => (),
            }
        }
    }

    fn skip_block_comment(&mut self) {
        let ch = self.advance();
        debug_assert_eq!(Some('#'), ch);
        let ch = self.advance();
        debug_assert_eq!(Some('|'), ch);

        let mut flag = false;
        while let Some(ch) = self.advance() {
            match ch {
                '|' => flag = true,
                '#' => if flag {break},
                _ => flag = false,
            }
        }
    }
}

fn is_whitespace(ch: char) -> bool {
    match ch {
        ' ' | '\t' | '\n' | '\r' => true,
        _ => false,
    }
}

fn is_identifier_initial(c: char) -> bool {
    match c {
        'a'..='z'
        | 'A'..='Z'
        | '!'
        | '$'
        | '%'
        | '&'
        | '*'
        | '/'
        | ':'
        | '<'
        | '='
        | '>'
        | '?'
        | '@'
        | '^'
        | '_'
        | '~' => true,
        _ => false,
    }
}

fn is_delimiter(c: char) -> bool {
    match c {
        ' ' | '\t' | '\n' | '\r' | '(' | ')' | '"' | ';' | '|' => true,
        _ => false,
    }
}

fn is_sign_subsequent(c: char) -> bool {
    match c {
        c if is_identifier_initial(c) => true,
        '+' | '-' | '@' => true,
        _ => false,
    }
}

fn move_location(ch: char, location: &mut Location) {
    match ch {
        '\n' => {
            location.row += 1;
            location.col = 0;
        },
        _ => location.col += 1,
    }
}

fn tokenize(text: &str) -> Result<Vec<Token>> {
    let mut iter = text.chars().peekable();
    let c = Lexer::new(&mut iter);
    Ok(c.collect::<Result<Vec<_>>>()?
        .into_iter()
        .map(|l| l.extract())
        .collect())
}




#[derive(Debug, Copy, Clone)]
enum Radix {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
}

impl Radix {
    pub fn from(radix: usize) -> Option<Radix> {
        match radix {
            2  => Some(Radix::Binary),
            8  => Some(Radix::Octal),
            10 => Some(Radix::Decimal),
            16 => Some(Radix::Hexadecimal),
            _  => None,
        }
    }

    pub fn contains(&self, ch: char) -> bool {
        match self {
            Radix::Binary  => '0' == ch || ch == '1',
            Radix::Octal   => '0' <= ch && ch <= '7',
            Radix::Decimal => '0' <= ch && ch <= '9',
            Radix::Hexadecimal => '0' <= ch && ch <= '9' || 'a' <= ch && ch <= 'f',
        }
    }
}

// Identifiers
// Identifiers have two uses within Scheme programs:
// - Any identifier can be used as a variable or as a syntactic keyword
// - When an identifier appears as a literal or within a literal, it is being used to denote a symbol.
#[test]
fn identifier() -> Result<()> {
    let tests = vec![
        // TODO
        // 2.1 inline hex escape
        // (r"|H\x65;llo|",    Token::Identifier(String::from("Hello"))),
        // (r"|\x3BB;|",       Token::Identifier(String::from("λ"))),
        // (r"|\x9;\x9;|",     Token::Identifier(String::from("\t\t"))),

        // 2.1 examples of identifiers
        ("...",             Token::Identifier(String::from("..."))),
        ("+",               Token::Identifier(String::from("+"))),
        ("+soup+",          Token::Identifier(String::from("+soup+"))),
        ("<=?",             Token::Identifier(String::from("<=?"))),
        ("->string",        Token::Identifier(String::from("->string"))),
        ("a34kTMNs",        Token::Identifier(String::from("a34kTMNs"))),
        ("lambda",          Token::Identifier(String::from("lambda"))),
        ("list->vector",    Token::Identifier(String::from("list->vector"))),
        ("q",               Token::Identifier(String::from("q"))),
        ("V17a",            Token::Identifier(String::from("V17a"))),
        ("|two words|",     Token::Identifier(String::from("two words"))),
        ("|two; words|",    Token::Identifier(String::from("two; words"))),
        ("the-word-recursion-has-many-meanings", Token::Identifier(String::from("the-word-recursion-has-many-meanings"))),

        // TODO
        // 2.1 case insensitive inline hex escapes 
        // (r"|\x3BB;|",       Token::Identifier(String::from("λ"))),
        // (r"|\x3bb;|",       Token::Identifier(String::from("λ"))),

        // TODO
        // 2.1 explicit control over case folding.



    ];

    let text = tests.iter().fold("".to_string(), |t, p| t + " " + p.0);
    let tokens = tests.iter().map(|p| p.1.clone()).collect::<Vec<_>>();

    assert_eq!(tokenize(&text)?, tokens);

    Ok(())
}

#[test]
// 2.2 Whitespace
// - Whitespace characters include the space, tab, and newline characters. 
// - Whitespace can occur between any two tokens, but not within a token.
// - Whitespace occurring inside a string or inside a symbol delimited by 
//   vertical lines is significant.
fn whitespace() -> Result<()> {
    let text = "( )\t.\n(\r)";
    assert_eq!(tokenize(text)?, vec![
        Token::LeftParen,
        Token::RightParen,
        Token::Period,
        Token::LeftParen,
        Token::RightParen,
    ]);

    let tests = vec![
       ("\" \t\r\n\"", Token::Primitive(Primitive::String(String::from(" \t\r\n")))),
       ("| \t\r\n|",   Token::Identifier(String::from(" \t\r\n"))),

    ];
    let text = tests.iter().fold("".to_string(), |t, p| t + " " + p.0);
    let tokens = tests.iter().map(|p| p.1.clone()).collect::<Vec<_>>();
    assert_eq!(tokenize(&text)?, tokens);

    Ok(())
}

// 2.2 Comments
// - A semicolon (;) indicates the start of a line comment. 
// - prefix a <datum> with #; and optional hwhitespacei.
// - Block comments are indicated with properly nested #| and |# pairs.
#[test]
fn comments() -> Result<()> {
    // A semicolon (;) indicates the start of a line comment. 
    let text = "; (( \n )) ; . .";
    assert_eq!(tokenize(text)?, vec![
        Token::RightParen,
        Token::RightParen,
    ]);

    // TODO
    // prefix a <datum> with #; and optional hwhitespacei.
    // let text = "#; (- 2 1) (+ a b)";
    // assert_eq!(tokenize(text)?, vec![
    //     Token::LeftParen,
    //     Token::Identifier(String::from("-")),
    //     Token::Identifier(String::from("a")),
    //     Token::Identifier(String::from("b")),
    //     Token::RightParen,
    // ]);

    // Block comments are indicated with properly nested #| and |# pairs.
    let text = "
        #|
        The FACT procedure computes the factorial
        of a non-negative integer.
        |#
        (define fact)
    ";
    assert_eq!(tokenize(text)?, vec![
        Token::LeftParen,
        Token::Identifier(String::from("define")),
        Token::Identifier(String::from("fact")),
        Token::RightParen,
    ]);

    Ok(())
}

// 2.3
// <token> ->  ( | ) | #( | #u8( | ’ | ` | , | ,@ | .
#[test]
fn other_notations() -> Result<()> {
    assert_eq!(
        tokenize("()#()#u8()'`,,@.")?,
        vec![
            Token::LeftParen,
            Token::RightParen,
            Token::VecConsIntro,
            Token::RightParen,
            Token::ByteVecConsIntro,
            Token::RightParen,
            Token::Quote,
            Token::Quasiquote,
            Token::Unquote,
            Token::UnquoteSplicing,
            Token::Period
        ]
    );
    Ok(())
}

#[test]
fn datum_labels() {
    // TODO
}

// 6.6
#[test]
fn characters() -> Result<()> {
    assert_eq!(
        tokenize("#\\a#\\ #\\\t")?,
        vec![
            Token::Primitive(Primitive::Character('a')),
            Token::Primitive(Primitive::Character(' ')),
            Token::Primitive(Primitive::Character('\t'))
        ]
    );
    Ok(())
}

// num R
#[test]
fn number() -> Result<()> {
    assert_eq!(
        tokenize(
            "
            +123 123 -123
            -123123/23
            3e-3
            3.3e+3
            .3e4
            3.e3
            
            1@-1
            1@0.0
            
            .1e-1+1.0i
            
            +nan.0-.1e-1i

            1/2+i

            -321-i

            .618033e+0-nan.0i

            +1/3i
            +.31415926e1i

            -10e-1i
            -2i

            +inf.0i
            -nan.0i

            +i

            -i
            "
        )?,
        vec![
            // case 1
            Complex::Real(Real::Integer(123)),
            Complex::Real(Real::Integer(123)),
            Complex::Real(Real::Integer(-123)),
            Complex::Real(Real::Ration(-123123, 23)),
            Complex::Real(Real::Float(0.003)),
            Complex::Real(Real::Float(3300.0)),
            Complex::Real(Real::Float(3000.0)),
            Complex::Real(Real::Float(3000.0)),

            // case 2
            Complex::Complex(Real::Integer(1), Real::Integer(-1)),
            Complex::Complex(Real::Integer(1), Real::Float(0.0)),

            // case 3
            Complex::Complex(Real::Float(0.01), Real::Float(1.0)),

            // case 4
            Complex::Complex(Real::PosNan, Real::Float(-0.01)),

            // case 5
            Complex::Complex(Real::Ration(1,2), Real::Integer(1)),

            // case 6
            Complex::Complex(Real::Integer(-321), Real::Integer(-1)),

            // case 7
            Complex::Complex(Real::Float(0.618033), Real::NegNan),

            // case 8
            Complex::Imaginary(Real::Ration(1, 3)),
            Complex::Imaginary(Real::Float(3.1415926)),

            // case 9
            Complex::Imaginary(Real::Float(-1.0)),
            Complex::Imaginary(Real::Integer(-2)),

            // case 10
            Complex::Imaginary(Real::PosInf),
            Complex::Imaginary(Real::NegNan),

            // case 11
            Complex::Imaginary(Real::Integer(1)),

            // case 12
            Complex::Imaginary(Real::Integer(-1)),

        ].iter().map(|c| Token::Primitive(Primitive::Complex(c.clone()))).collect::<Vec<_>>()
    );

    Ok(())
}

#[test]
fn string() -> Result<()> {
    assert_eq!(
        tokenize("\"()+-123\"\"\\\"\"\"\\a\\b\\t\\r\\n\\\\\\|\"")?,
        vec![
            Token::Primitive(Primitive::String(String::from("()+-123"))),
            Token::Primitive(Primitive::String(String::from("\""))),
            Token::Primitive(Primitive::String(String::from("\u{007}\u{008}\t\r\n\\|")))
        ]
    );
    Ok(())
}

#[test]
fn delimiter() -> Result<()> {
    
    assert_eq!(
        tokenize("\t(- \n4\r(+ 1 2)) ...)")?,
        vec![
            Token::LeftParen,
            Token::Identifier(String::from("-")),
            Token::Primitive(Primitive::Complex(Complex::Real(Real::Integer(4)))),
            Token::LeftParen,
            Token::Identifier(String::from("+")),
            Token::Primitive(Primitive::Complex(Complex::Real(Real::Integer(1)))),
            Token::Primitive(Primitive::Complex(Complex::Real(Real::Integer(2)))),
            Token::RightParen,
            Token::RightParen,
            Token::Identifier(String::from("...")),
            Token::RightParen,
        ]
    );
    Ok(())
}

#[test]
fn script() -> Result<()> {
    assert_eq!(
        tokenize(
            "(define-syntax begin
            (syntax-rules ()
                ((begin exp ... )
                    ((lambda () exp ... )))))"
        )?,
        vec![
            Token::LeftParen,
            Token::Identifier("define-syntax".to_string()),
            Token::Identifier("begin".to_string()),
            Token::LeftParen,
            Token::Identifier("syntax-rules".to_string()),
            Token::LeftParen,
            Token::RightParen,
            Token::LeftParen,
            Token::LeftParen,
            Token::Identifier("begin".to_string()),
            Token::Identifier("exp".to_string()),
            Token::Identifier("...".to_string()),
            Token::RightParen,
            Token::LeftParen,
            Token::LeftParen,
            Token::Identifier("lambda".to_string()),
            Token::LeftParen,
            Token::RightParen,
            Token::Identifier("exp".to_string()),
            Token::Identifier("...".to_string()),
            Token::RightParen,
            Token::RightParen,
            Token::RightParen,
            Token::RightParen,
            Token::RightParen,
        ]
    );
    Ok(())
}

