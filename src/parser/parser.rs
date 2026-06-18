use crate::parser::{
    lexer::Lexer,
    token::{
        IndexedToken,
        Token::{self, *},
    },
    types::{AccessModifier, ImportObject, JavaFile, ParseErr, ParseResult, QualifiedName, Type},
};
pub struct Parser<'a> {
    pub(super) string: &'a str,
    tokens: Vec<IndexedToken<'a>>,
    ind: usize,
    lookahead: Option<IndexedToken<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(s: &'a str) -> Result<Self, ParseErr<'a>> {
        let mut lexer = Lexer::new(s);
        let mut tokens = vec![];
        let mut ind: usize;

        loop {
            if let Some(token) = lexer.get_next_indexed_token() {
                tokens.push(token);
                if tokens.last().unwrap().token == EOF {
                    ind = tokens.last().unwrap().addr;
                    break;
                }
            } else {
                return Err(ParseErr::LexerError);
            }
        }

        // essentially idea is that we are going to look at,
        // at most, 4 values, and counting the first EOF,
        // it is suffice to say that we insert 3 more EOF's
        // so that we don't run into Out of Bounds err.
        let mut gen_eof = || {
            ind += 1;
            IndexedToken {
                token: EOF,
                addr: ind,
                len: 1,
            }
        };
        for _ in 0..3 {
            tokens.push(gen_eof());
        }

        Ok(Self {
            string: s,
            tokens,
            ind: 0,
            lookahead: None,
        })
    }

    // Parse the java file, return the structure of the file which can be
    // thought of as a specialized AST
    pub fn parse(&mut self) -> Result<JavaFile<'a>, ParseErr<'a>> {
        self.java_file()
    }

    pub(super) fn get_next_token(&mut self) -> Result<IndexedToken<'a>, ParseErr<'a>> {
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

    pub(super) fn get_current_token(&mut self) -> ParseResult<'a, IndexedToken<'a>> {
        self.tokens
            .get(self.ind - 1)
            .ok_or(ParseErr::IndexingError)
            .copied()
    }
    pub(super) fn peek_next_token(&self) -> Result<IndexedToken<'a>, ParseErr<'a>> {
        if let Some(token) = self.lookahead {
            Ok(token)
        } else {
            self.tokens
                .get(self.ind)
                .copied()
                .ok_or(ParseErr::UnexpectedEOF)
        }
    }

    pub(super) fn peek_token_offset(
        &self,
        offset: usize,
    ) -> Result<IndexedToken<'a>, ParseErr<'a>> {
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

    pub(super) fn consume_gt(&mut self) -> ParseResult<'a, ()> {
        let indexed_token = self.get_next_token()?;
        match indexed_token.token {
            GreaterThan => Ok(()),
            Op(">=") => {
                self.lookahead = Some(IndexedToken {
                    token: Assignment("="),
                    addr: indexed_token.addr + 1,
                    len: indexed_token.len - 1,
                });
                Ok(())
            }
            Op(">>") => {
                self.lookahead = Some(IndexedToken {
                    token: GreaterThan,
                    addr: indexed_token.addr + 1,
                    len: indexed_token.len - 1,
                });
                Ok(())
            }
            Assignment(">>=") => {
                self.lookahead = Some(IndexedToken {
                    token: Op(">="),
                    addr: indexed_token.addr + 1,
                    len: indexed_token.len - 1,
                });
                Ok(())
            }
            Op(">>>") => {
                self.lookahead = Some(IndexedToken {
                    token: Op(">>"),
                    addr: indexed_token.addr + 1,
                    len: indexed_token.len - 1,
                });
                Ok(())
            }
            Assignment(">>>=") => {
                self.lookahead = Some(IndexedToken {
                    token: Assignment(">>="),
                    addr: indexed_token.addr + 1,
                    len: indexed_token.len - 1,
                });
                Ok(())
            }
            token => Err(ParseErr::UnexpectedToken {
                expected: ">",
                got: vec![token],
            }),
        }
    }

    pub(super) fn skip_brace(
        &mut self,
        open_brace: Token,
        close_brace: Token,
    ) -> ParseResult<'a, ()> {
        if self.get_next_token()?.token != open_brace {
            return Err(ParseErr::UnexpectedToken {
                expected: "LBrace | LBracket | LParen",
                got: vec![self.get_current_token()?.token],
            });
        }

        let mut stack: usize = 1;
        while stack > 0 {
            match self.get_next_token()?.token {
                token if token == open_brace => stack += 1,
                token if token == close_brace => stack -= 1,
                EOF => return Err(ParseErr::UnexpectedEOF),
                _ => {}
            }
        }

        Ok(())
    }

    /// a valid assignment-ending comma is the first comma of the following phrase:
    /// ```
    /// "," <annotations> IDENTIFIER {"," <annotations> IDENTIFIER} (";" | "=")
    /// ```
    ///
    /// Note: Called after the comma is consumed.
    pub(super) fn check_end_assignment_comma(&mut self) -> ParseResult<'a, bool> {
        if self.peek_next_token()?.token != Comma {
            return Err(ParseErr::UnexpectedToken {
                expected: "Comma",
                got: vec![self.get_current_token()?.token],
            });
        }

        let mut offset: usize = 1;

        // <annotations>
        offset = self.skip_annotations(offset)?;

        // IDENTIFIER
        let Identifier(_) = self.peek_token_offset(offset)?.token else {
            return Ok(false);
        };
        offset += 1;

        // {"," <annotations> IDENTIFIER}
        while self.peek_token_offset(offset)?.token == Comma {
            // ","
            offset += 1;

            // <annotations>
            offset = self.skip_annotations(offset)?;

            let Identifier(_) = self.peek_token_offset(offset)?.token else {
                return Ok(false);
            };
            offset += 1;
        }

        match self.peek_token_offset(offset)?.token {
            Semicolon | Assignment("=") => Ok(true),
            _ => Ok(false),
        }
    }

    /// return the index after skipping annotation
    pub(super) fn skip_annotations(&self, offset: usize) -> ParseResult<'a, usize> {
        let mut offset = offset;

        // go through annotations
        while self.peek_token_offset(offset)?.token == At {
            // "@"
            offset += 1;

            // "@" ID {"." ID}[
            //  | ("("...")")
            //  | ("{"..."}")
            // ]

            // ID
            let Identifier(_) = self.peek_token_offset(offset)?.token else {
                return Err(ParseErr::UnexpectedToken {
                    expected: "IDENTIFIER",
                    got: vec![self.peek_token_offset(offset)?.token],
                });
            };
            offset += 1;

            // {"." ID}
            while self.peek_token_offset(offset)?.token == Dot {
                // read Dot
                offset += 1;

                // read Identifier
                let Identifier(_) = self.peek_token_offset(offset)?.token else {
                    return Err(ParseErr::UnexpectedToken {
                        expected: "IDENTIFIER",
                        got: vec![self.peek_token_offset(offset)?.token],
                    });
                };
                offset += 1;
            }

            offset = match self.peek_token_offset(offset)?.token {
                LBrace => self.skip_brace_peek_forward(LBrace, RBrace, offset)?,
                LParen => self.skip_brace_peek_forward(LParen, RParen, offset)?,
                _ => offset,
            };
        }

        Ok(offset)
    }

    /// returns the offset just after the closing of the brace
    pub(super) fn skip_brace_peek_forward(
        &self,
        open_brace: Token,
        close_brace: Token,
        offset: usize,
    ) -> ParseResult<'a, usize> {
        let mut offset = offset;

        if self.peek_token_offset(offset)?.token != open_brace {
            return Err(ParseErr::UnexpectedToken {
                expected: "Open brace/bracket/paren",
                got: vec![self.peek_token_offset(offset)?.token],
            });
        }
        offset += 1;
        let mut stack = 1;

        while stack > 0 {
            match self.peek_token_offset(offset)?.token {
                EOF => return Err(ParseErr::UnexpectedEOF),
                token if token == open_brace => stack += 1,
                token if token == close_brace => stack -= 1,
                _ => {}
            }
            offset += 1;
        }

        Ok(offset)
    }
}

// ---------------------------------------------------------------------
// --------------------- Parsing ---------------------------------------
// ---------------------------------------------------------------------

// Parsing
impl<'a> Parser<'a> {
    /// `<java_file> ::= [<package_decl>] <import> {<type_decl>}`
    fn java_file(&mut self) -> Result<JavaFile<'a>, ParseErr<'a>> {
        let mut file = JavaFile {
            package_name: None,
            imported_objects: vec![],
            type_decls: vec![],
        };

        // <package_decl>
        if self.peek_next_token()?.token == Keyword("package") {
            file.package_name = Some(self.package_decl()?);
        }

        // <import>
        file.imported_objects.append(&mut self.import()?);

        // {<type_decl>}
        let mut have_public_type = false;
        while let Ok(token) = self.peek_next_token()
            && token.token != EOF
        {
            file.type_decls.push(self.type_decl(QualifiedName(vec![]))?);
            if file.type_decls.last().unwrap().modifiers.access_modifier == AccessModifier::Public {
                if have_public_type {
                    return Err(ParseErr::MultiplePublicTypesError);
                } else {
                    have_public_type = true;
                }
            }
        }

        Ok(file)
    }

    /// `<package_decl>  ::= [ "package" <qualified_name> ";" ]`
    fn package_decl(&mut self) -> ParseResult<'a, QualifiedName<'a>> {
        if self.get_next_token()?.token != Keyword("package") {
            Err(ParseErr::UnexpectedToken {
                expected: "package",
                got: vec![self.get_current_token()?.token],
            })
        } else {
            self.qualified_name()
        }
    }

    /// `<import> ::= { "import" ["static"] <qualified_name>[.*] ";" }`
    fn import(&mut self) -> ParseResult<'a, Vec<ImportObject<'a>>> {
        let mut v: Vec<ImportObject> = vec![];

        loop {
            if self.peek_next_token()?.token != Keyword("import") {
                break;
            }

            self.get_next_token()?;
            let is_static = if self.peek_next_token()?.token == Keyword("static") {
                self.get_next_token()?;
                true
            } else {
                false
            };

            let name = self.qualified_name()?;

            let is_wildcard = match (
                self.peek_token_offset(0)?.token,
                self.peek_token_offset(1)?.token,
            ) {
                (Dot, Op("*")) => {
                    self.get_next_token()?;
                    self.get_next_token()?;
                    true
                }
                _ => false,
            };

            if self.get_next_token()?.token != Semicolon {
                return Err(ParseErr::UnexpectedToken {
                    expected: "Semicolon",
                    got: vec![self.get_current_token()?.token],
                });
            }

            v.push(ImportObject {
                name,
                is_static,
                is_wildcard,
            });
        }

        Ok(v)
    }

    /// `<type_decl> ::= {<annotation>} <modifiers> ( <enum_decl> | <class_decl> |
    /// <interface_decl> | <annotation_decl> )`
    fn type_decl(&mut self, prefix: QualifiedName<'a>) -> ParseResult<'a, Type<'a>> {
        // {<annotation>}
        let annotation = self.annotations()?;

        // <modifiers>
        let modifiers = self.modifiers()?;

        // (<enum_decl> | <class_decl> | <interface_decl> | <annotation_decl>)
        let mut typeclass = match self.peek_next_token()?.token {
            Keyword("class") => self.class_decl(prefix)?,
            Keyword("interface") => self.interface_decl(prefix)?,
            Keyword("enum") => self.enum_decl(prefix)?,
            At => self.annotation_decl(prefix)?,
            token => {
                return Err(ParseErr::UnexpectedToken {
                    expected: "class | interface | enum | @",
                    got: vec![token],
                });
            }
        };
        typeclass.modifiers = modifiers;
        typeclass.annotation = annotation;

        Ok(typeclass)
    }

    // ---------------------------------------------------------------------
    // ----------------------- Enum Nonterminals ---------------------------
    // ---------------------------------------------------------------------

    pub(crate) fn enum_decl(&mut self, prefix: QualifiedName<'a>) -> ParseResult<'a, Type<'a>> {
        Err(ParseErr::UnimplementedError)
    }

    // ---------------------------------------------------------------------
    // ----------------------- Interface Nonterminals ----------------------
    // ---------------------------------------------------------------------
    pub(crate) fn interface_decl(
        &mut self,
        prefix: QualifiedName<'a>,
    ) -> ParseResult<'a, Type<'a>> {
        Err(ParseErr::UnimplementedError)
    }

    // ---------------------------------------------------------------------
    // ----------------------- Annotation Nonterminals ---------------------
    // ---------------------------------------------------------------------

    pub(crate) fn annotation_decl(
        &mut self,
        prefix: QualifiedName<'a>,
    ) -> ParseResult<'a, Type<'a>> {
        Err(ParseErr::UnimplementedError)
    }
}

// ---------------------------------------------------------------------
// ------------------------ TESTS --------------------------------------
// ---------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_comma_end_property() {
        let mut parser = Parser::new(
            ", @annotation1 val1, @annotation2(v1, v2) val2, @annotation3{v1 = a1, v2 = a2} val3 = true;"
        ).unwrap();
        assert!(parser.check_end_assignment_comma().unwrap());

        parser = Parser::new(
            ", @annotation1 val1, @annotation2(v1, v2) val2, @annotation3{v1 = a1, v2 = a2} val3, val4;",
        )
        .unwrap();
        assert!(parser.check_end_assignment_comma().unwrap());

        parser = Parser::new(
            "<v1.v2, @annotation1 val1, @annotation2(v1, v2) val2, @annotation3{v1 = a1, v2 = a2} val3>",
        )
        .unwrap();
        parser.get_next_token().unwrap();
        parser.qualified_name().unwrap();
        assert!(!parser.check_end_assignment_comma().unwrap());
    }
}
