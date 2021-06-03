use super::Expression;

use std::io::Read;

pub struct Parser {
    unget_stack: Vec<char>,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            unget_stack: Vec::new(),
        }
    }

    pub fn parse<T: Read>(&mut self, mut reader: T) -> Result<Vec<Expression>, ()> {
        let mut res = vec![];
        loop {
            // match self.try_parse_one_expression(reader) {
            //     Err(e) => return Err(e),
            //     Ok(None) => break,
            //     Ok(Some(exp)) => res.push(exp),
            // }
        }
        Ok(res)
    }

    fn try_parse_one_expression<T: Read>(&mut self, mut reader: T) -> Result<Option<Expression>, ()> {
        if let Some(ch) = self.get_char(reader, true) {
            Ok(None)
        } else {
            Ok(None)
        }
    }

    fn get_char<T: Read>(&mut self, mut reader: T, skip_whitespace: bool) -> Option<char> {
        loop {
            match self.unget_stack.pop() {
                Some(e) if !e.is_whitespace() || !skip_whitespace => return Some(e),
                Some(_) => continue,
                None => ()
            };

            let mut one_char_buffer = [0];
            let n_bytes_read = reader.read(&mut one_char_buffer);
            match n_bytes_read {
                Ok(0) => return None,
                Ok(1) => (),
                Ok(_) => panic!("unsupposed to be here"),
                Err(e) => return None,
            };
            let ch = one_char_buffer[0] as char;

            match ch {
                c if !c.is_whitespace() || !skip_whitespace => return Some(c),
                _  => (),
            };
        }
    }
}

