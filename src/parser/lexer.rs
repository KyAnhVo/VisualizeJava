use crate::parser::token::{
    Keyword::*,
    Token::{self, *},
};
use std::{fs, io};
struct Lexer {
    str: Vec<char>,
    ind: usize,
    curr_char: Option<char>,
}

impl Lexer {
    pub fn new(file: String) -> io::Result<Self> {
        Ok(Self {
            str: fs::read_to_string(file)?.chars().collect(),
            ind: 0,
            curr_char: Some('\0'),
        })
    }

    pub fn get_next_token(&mut self) -> Option<Token> {
        loop {
            let next_char = self.get_next_char();
            if next_char.is_none() {
                return Some(EOF);
            }
            match *self.get_next_char()? {
                '{' => return Some(LBrace),
                '}' => return Some(RBrace),
                '[' => return Some(LBracket),
                ']' => return Some(RBracket),
                '(' => return Some(LParen),
                ')' => return Some(RParen),
                '?' => return Some(QuestionMark),
                '@' => return Some(Annotation),
                ',' => return Some(Comma),
                '~' => return Some(Op("~")),
                '.' => {
                    if self.peek_next_2() == (Some(&'.'), Some(&'.')) {
                        self.get_next_char();
                        self.get_next_char();
                        return Some(Op("..."));
                    } else {
                        return Some(Dot);
                    }
                }
                _ => return None,
            }
        }
    }

    fn get_next_char(&mut self) -> Option<&char> {
        self.ind += 1;
        self.str.get(self.ind - 1)
    }

    fn peek_next_char(&self) -> Option<&char> {
        self.str.get(self.ind)
    }

    fn peek_char_offset(&self, offset: usize) -> Option<&char> {
        self.str.get(self.ind + offset)
    }

    fn peek_next_2(&self) -> (Option<&char>, Option<&char>) {
        (self.peek_char_offset(0), self.peek_char_offset(1))
    }

    fn peek_next_3(&self) -> (Option<&char>, Option<&char>, Option<&char>) {
        (
            self.peek_char_offset(0),
            self.peek_char_offset(1),
            self.peek_char_offset(2),
        )
    }
}
