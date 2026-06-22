use crate::parser::token::{
    IndexedToken,
    Token::{self, *},
};
pub struct Lexer<'a> {
    s: &'a str,
    ind: usize,
    curr_char: Option<char>,
    token_start: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            s,
            ind: 0,
            curr_char: Some('\0'),
            token_start: 0,
        }
    }

    pub fn get_next_indexed_token(&mut self) -> Option<IndexedToken<'a>> {
        let token = self.get_next_token()?;

        let len = self.ind - self.token_start;
        Some(IndexedToken {
            token,
            addr: self.token_start,
            len,
        })
    }
    pub fn get_next_token(&mut self) -> Option<Token<'a>> {
        loop {
            self.token_start = self.ind;
            let next_char = self.get_next_char();
            if next_char.is_none() {
                return Some(EOF);
            }
            match next_char? {
                c if c.is_whitespace() => continue,
                '{' => return Some(LBrace),
                '}' => return Some(RBrace),
                '[' => return Some(LBracket),
                ']' => return Some(RBracket),
                '(' => return Some(LParen),
                ')' => return Some(RParen),
                '?' => return Some(QuestionMark),
                '@' => return Some(At),
                ',' => return Some(Comma),
                '~' => return Some(Op("~")),
                ';' => return Some(Semicolon),
                '.' => {
                    if self.peek_next_2() == (Some('.'), Some('.')) {
                        self.get_next_char();
                        self.get_next_char();
                        return Some(Op("..."));
                    } else {
                        return Some(Dot);
                    }
                }
                c @ ('^' | '%' | '!') => match self.peek_next_char() {
                    Some('=') => {
                        self.get_next_char();
                        return Some(match c {
                            '^' => Assignment("^="),
                            '%' => Assignment("%="),
                            '!' => Op("!="),
                            _ => unreachable!(),
                        });
                    }
                    _ => {
                        return Some(Op(match c {
                            '^' => "^",
                            '%' => "%",
                            '!' => "!",
                            _ => unreachable!(),
                        }));
                    }
                },
                ':' => match self.peek_next_char() {
                    Some(':') => {
                        self.get_next_char();
                        return Some(Op("::"));
                    }
                    _ => return Some(Op(":")),
                },
                '=' => match self.peek_next_char() {
                    Some('=') => {
                        self.get_next_char();
                        return Some(Op("=="));
                    }
                    _ => return Some(Assignment("=")),
                },
                '+' => match self.peek_next_char() {
                    Some('=') => {
                        self.get_next_char();
                        return Some(Assignment("+="));
                    }
                    Some('+') => {
                        self.get_next_char();
                        return Some(Op("++"));
                    }
                    _ => return Some(Op("+")),
                },
                '-' => match self.peek_next_char() {
                    Some('=') => {
                        self.get_next_char();
                        return Some(Assignment("-="));
                    }
                    Some('-') => {
                        self.get_next_char();
                        return Some(Op("--"));
                    }
                    _ => return Some(Op("-")),
                },
                '*' => match self.peek_next_char() {
                    Some('=') => {
                        self.get_next_char();
                        return Some(Assignment("*="));
                    }
                    _ => return Some(Op("*")),
                },
                '/' => match self.peek_next_char() {
                    Some('/') => self.pass_inline_comment(),
                    Some('*') => {
                        self.get_next_char();
                        self.pass_block_comment();
                    }
                    Some('=') => {
                        self.get_next_char();
                        return Some(Assignment("/="));
                    }
                    _ => return Some(Op("/")),
                },
                '&' => match self.peek_next_char() {
                    Some('=') => {
                        self.get_next_char();
                        return Some(Assignment("&="));
                    }
                    Some('&') => {
                        self.get_next_char();
                        return Some(Op("&&"));
                    }
                    _ => return Some(Op("&")),
                },
                '|' => match self.peek_next_char() {
                    Some('=') => {
                        self.get_next_char();
                        return Some(Assignment("|="));
                    }
                    Some('|') => {
                        self.get_next_char();
                        return Some(Op("||"));
                    }
                    _ => return Some(Op("|")),
                },
                '>' => match self.peek_next_3() {
                    (Some('>'), Some('>'), Some('=')) => {
                        self.get_next_char();
                        self.get_next_char();
                        self.get_next_char();
                        return Some(Assignment(">>>="));
                    }
                    (Some('>'), Some('>'), _) => {
                        self.get_next_char();
                        self.get_next_char();
                        return Some(Op(">>>"));
                    }
                    (Some('>'), Some('='), _) => {
                        self.get_next_char();
                        self.get_next_char();
                        return Some(Assignment(">>="));
                    }
                    (Some('>'), _, _) => {
                        self.get_next_char();
                        return Some(Op(">>"));
                    }
                    (Some('='), _, _) => {
                        self.get_next_char();
                        return Some(Op(">="));
                    }
                    _ => return Some(GreaterThan),
                },
                '<' => match self.peek_next_2() {
                    (Some('<'), Some('=')) => {
                        self.get_next_char();
                        self.get_next_char();
                        return Some(Assignment("<<="));
                    }
                    (Some('<'), _) => {
                        self.get_next_char();
                        return Some(Op("<<"));
                    }
                    (Some('='), _) => {
                        self.get_next_char();
                        return Some(Op("<="));
                    }
                    _ => return Some(LessThan),
                },
                '\'' => return self.get_char_literal(),
                '\"' => return self.get_string_literal(),
                c if c.is_alphanumeric() => {
                    let s = self.get_identifier_chain();
                    if matches!(
                        s,
                        "abstract"
                            | "class"
                            | "const"
                            | "default"
                            | "enum"
                            | "extends"
                            | "final"
                            | "implements"
                            | "import"
                            | "interface"
                            | "package"
                            | "public"
                            | "private"
                            | "static"
                            | "synchronized"
                            | "transient"
                            | "return"
                            | "void"
                            | "if"
                            | "else"
                            | "while"
                            | "for"
                            | "new"
                            | "this"
                            | "super"
                            | "break"
                            | "continue"
                            | "case"
                            | "try"
                            | "catch"
                            | "do"
                            | "finally"
                            | "instanceof"
                            | "native"
                            | "protected"
                            | "switch"
                            | "throw"
                            | "throws"
                            | "volatile"
                            | "sealed"
                            | "permits"
                    ) {
                        return Some(Keyword::<'a>(s));
                    } else {
                        return Some(Identifier::<'a>(s));
                    }
                }
                _ => return None,
            }
        }
    }

    fn get_identifier_chain(&mut self) -> &'a str {
        let start_ind = self.ind - self.curr_char.unwrap().len_utf8();
        let mut iter = self.s[self.ind..].chars();
        loop {
            if let Some(c) = iter.next() {
                if c.is_alphanumeric() || c == '_' || c == '$' {
                    self.ind += c.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        return &self.s[start_ind..self.ind];
    }

    fn get_char_literal(&mut self) -> Option<Token<'a>> {
        let start_index = self.ind - 1;
        let mut iter = self.s[self.ind..].chars();
        loop {
            if let Some(mut c) = iter.next() {
                self.ind += c.len_utf8();
                if c == '\\' {
                    // consume the next char too
                    c = iter.next()?;
                    self.ind += c.len_utf8();
                } else if c == '\'' {
                    break;
                }
            } else {
                return None;
            }
        }
        Some(Literal(&self.s[start_index..self.ind]))
    }

    fn get_string_literal(&mut self) -> Option<Token<'a>> {
        let start_index = self.ind - 1;
        let mut iter = self.s[self.ind..].chars();
        loop {
            if let Some(mut c) = iter.next() {
                self.ind += c.len_utf8();
                if c == '\\' {
                    c = iter.next()?;
                    self.ind += c.len_utf8();
                } else if c == '\"' {
                    break;
                }
            }
        }
        Some(Literal(&self.s[start_index..self.ind]))
    }
    fn pass_inline_comment(&mut self) {
        let mut c = self.get_next_char();
        while c != None && c != Some('\n') {
            c = self.get_next_char();
        }
    }

    fn pass_block_comment(&mut self) {
        loop {
            match (self.get_next_char(), self.peek_next_char()) {
                (None, _) | (_, None) => return,
                (Some('*'), Some('/')) => {
                    self.get_next_char();
                    return;
                }
                _ => {}
            };
        }
    }
    fn get_next_char(&mut self) -> Option<char> {
        self.curr_char = self.s[self.ind..].chars().next();
        self.ind += self.curr_char?.len_utf8();
        self.curr_char
    }

    fn peek_next_char(&self) -> Option<char> {
        self.s[self.ind..].chars().next()
    }

    fn peek_next_2(&self) -> (Option<char>, Option<char>) {
        let mut s = self.s[self.ind..].chars();
        (s.next(), s.next())
    }

    fn peek_next_3(&self) -> (Option<char>, Option<char>, Option<char>) {
        let mut s = self.s[self.ind..].chars();
        (s.next(), s.next(), s.next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_lexer(lexer: &mut Lexer, token: Token) {
        assert_eq!(lexer.get_next_token().unwrap(), token);
    }

    #[test]
    fn test_str_lit() {
        let mut lexer = Lexer::new("\"a\\n\\r\\\\a\"");
        lexer.get_next_char();
        let s = lexer.get_string_literal();
        assert!(s.unwrap() == Literal("\"a\\n\\r\\\\a\""));
    }

    #[test]
    fn test_char_lit() {
        let mut lexer = Lexer::new("'\\r'");
        lexer.get_next_char();
        let s = lexer.get_char_literal();
        assert!(s.unwrap() == Literal("'\\r'"))
    }

    #[test]
    fn test_long_stuff() {
        let mut lexer = Lexer::new("String s = \"Hello World!\";");
        assert!(lexer.get_next_token().unwrap() == Token::Identifier("String"));
        assert!(lexer.get_next_token().unwrap() == Token::Identifier("s"));
        assert!(lexer.get_next_token().unwrap() == Token::Assignment("="));
        assert!(lexer.get_next_token().unwrap() == Token::Literal("\"Hello World!\""));
        assert!(lexer.get_next_token().unwrap() == Token::Semicolon);
        assert!(lexer.get_next_token().unwrap() == Token::EOF);
    }

    #[test]
    fn test_keyword_and_identifier() {
        let mut lexer = Lexer::new("public class Integer { private int i; }");
        let mut lex = |token: Token| test_lexer(&mut lexer, token);
        lex(Keyword("public"));
        lex(Keyword("class"));
        lex(Identifier("Integer"));
        lex(LBrace);
        lex(Keyword("private"));
        lex(Identifier("int"));
        lex(Identifier("i"));
        lex(Semicolon);
        lex(RBrace);
    }
}
