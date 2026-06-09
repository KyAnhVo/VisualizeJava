use crate::parser::{
    self,
    lexer::Lexer,
    token::Token::{self, *},
    types::{self, JavaFile, ParseErr, QualifiedName, RefType, TypeArg, TypeArgList},
};
pub struct Parser<'a> {
    tokens: Vec<Token<'a>>,
    ind: usize,
    lookahead: Option<Token<'a>>,
}

// Parsing
impl<'a> Parser<'a> {
    pub fn parse(&mut self) -> parser::Result<'a, JavaFile<'a>> {
        self.java_file()
    }
    fn java_file(&mut self) -> parser::Result<'a, JavaFile<'a>> {
        let file = JavaFile {
            package_name: None,
            imported_objects: vec![],
            type_decls: vec![],
        };

        Ok(file)
    }

    // ---------------------------------------------------------------------
    // ------------------- Util nonterms -----------------------------------
    // ---------------------------------------------------------------------

    fn qualified_name(&mut self) -> parser::Result<'a, QualifiedName<'a>> {
        Err(ParseErr::UnimplementedError)
    }
}

// helpers for Parser
impl<'a> Parser<'a> {
    pub fn new(s: &'a str) -> types::Result<'a, Self> {
        let mut lexer = Lexer::new(s);
        let mut tokens = vec![];

        loop {
            if let Some(token) = lexer.get_next_token() {
                tokens.push(token);
                if tokens.last().unwrap() == &EOF {
                    break;
                }
            } else {
                return Err(ParseErr::LexerError);
            }
        }

        tokens.push(EOF);
        tokens.push(EOF);
        tokens.push(EOF);

        Ok(Self {
            tokens,
            ind: 0,
            lookahead: None,
        })
    }

    fn get_next_token(&mut self) -> parser::Result<'a, Token<'a>> {
        if let Some(token) = self.lookahead.take() {
            Ok(token)
        } else {
            self.ind += 1;
            self.tokens
                .get(self.ind - 1)
                .copied()
                .ok_or(ParseErr::UnexpectedEOF)
        }
    }

    fn peek_next_token(&self) -> parser::Result<'a, Token<'a>> {
        if let Some(token) = self.lookahead {
            Ok(token)
        } else {
            self.tokens
                .get(self.ind)
                .copied()
                .ok_or(ParseErr::UnexpectedEOF)
        }
    }

    fn peek_token_offset(&self, offset: usize) -> parser::Result<'a, Token<'a>> {
        if offset == 0 {
            self.peek_next_token()
        } else {
            if self.lookahead != None {
                self.tokens
                    .get(self.ind + offset - 1)
                    .copied()
                    .ok_or(ParseErr::UnexpectedEOF)
            } else {
                self.tokens
                    .get(self.ind + offset)
                    .copied()
                    .ok_or(ParseErr::UnexpectedEOF)
            }
        }
    }
}
