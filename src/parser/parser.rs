use crate::parser::{
    self,
    lexer::Lexer,
    token::{
        IndexedToken,
        Token::{self, *},
    },
    types::{
        AccessModifier, ImportObject, JavaFile, Modifiers, ParseErr, ParseResult, QualifiedName,
        RefType, TypeArg, TypeArgList,
    },
};
pub struct Parser<'a> {
    string: &'a str,
    tokens: Vec<IndexedToken<'a>>,
    ind: usize,
    lookahead: Option<IndexedToken<'a>>,
}

// Parsing
impl<'a> Parser<'a> {
    pub fn parse(&mut self) -> Result<JavaFile<'a>, ParseErr<'a>> {
        self.java_file()
    }

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
            })
        }

        Ok(v)
    }

    // ---------------------------------------------------------------------
    // ------------------- Util nonterms -----------------------------------
    // ---------------------------------------------------------------------

    /// `<qualified_name> ::= IDENTIFIER {"." IDENTIFIER}`
    fn qualified_name(&mut self) -> ParseResult<'a, QualifiedName<'a>> {
        let mut name = QualifiedName(vec![]);

        // IDENTIFIER
        let token = self.get_next_token()?;
        if let Identifier(s) = token.token {
            name.0.push(s);
        } else {
            return Err(ParseErr::UnexpectedToken {
                expected: "IDENTIFIER",
                got: vec![token.token],
            });
        }

        // {"." IDENTIFIER}
        loop {
            match (
                self.peek_token_offset(0)?.token,
                self.peek_token_offset(1)?.token,
            ) {
                (Dot, Identifier(s)) => {
                    name.0.push(s);
                    self.get_next_token()?;
                    self.get_next_token()?;
                }
                _ => break,
            }
        }

        Ok(name)
    }

    /// `<annotation> ::= "@" <qualified_name> [( "(" <skip_parens> ")" )| ( "{" <skip_brace> "}"
    /// )]`
    fn annotation(&mut self) -> ParseResult<'a, &'a str> {
        if self.get_next_token()?.token != Annotation {
            return Err(ParseErr::UnexpectedToken {
                expected: "@",
                got: vec![self.get_current_token()?.token],
            });
        }

        let start_ind = self.get_current_token()?.addr;

        // <qualified_name>
        self.qualified_name()?;

        // [( "(" <skip_parens> ")" )| ( "{" <skip_brace> "}" )]
        match self.peek_next_token()?.token {
            LBrace => {
                self.get_next_token()?;
                let mut stack = 1;
                while stack > 0 {
                    match self.get_next_token()?.token {
                        LBrace => stack += 1,
                        RBrace => stack -= 1,
                        _ => {}
                    }
                }
            }
            LParen => {
                self.get_next_token()?;
                let mut stack = 1;
                while stack > 0 {
                    match self.get_next_token()?.token {
                        LParen => stack += 1,
                        RParen => stack -= 1,
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        let len = (self.get_current_token()?.addr + self.get_current_token()?.len) - start_ind;
        return Ok(&self.string[start_ind..start_ind + len]);
    }

    /// `<modifiers> ::= { "public" | "private" | "protected" | "abstract" | "static" | "final" |
    /// "strictfp" }`
    pub fn modifiers(&mut self) -> ParseResult<'a, Modifiers<'a>> {
        let mut modifiers = Modifiers {
            modifiers: vec![],
            access_modifier: AccessModifier::Default,
        };

        while let Keyword(s) = self.peek_next_token()?.token {
            if matches!(s, "public" | "private" | "protected") {
                if modifiers.access_modifier != AccessModifier::Default {
                    return Err(ParseErr::UnexpectedToken {
                        expected: "non-access modifier",
                        got: vec![Keyword(s)],
                    });
                }
                modifiers.access_modifier = match s {
                    "public" => AccessModifier::Public,
                    "private" => AccessModifier::Private,
                    "protected" => AccessModifier::Protected,
                    _ => unreachable!("we have guaranteed public/private/protected"),
                };
            } else if !matches!(s, "abstract" | "static" | "final" | "strictfp") {
                break;
            }
            self.get_next_token()?;
            modifiers.modifiers.push(s);
        }

        Ok(modifiers)
    }
}

// helpers for Parser
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

        let mut gen_eof = || {
            ind += 1;
            IndexedToken {
                token: EOF,
                addr: ind,
                len: 1,
            }
        };

        tokens.push(gen_eof());
        tokens.push(gen_eof());
        tokens.push(gen_eof());

        Ok(Self {
            string: s,
            tokens,
            ind: 0,
            lookahead: None,
        })
    }

    fn get_next_token(&mut self) -> Result<IndexedToken<'a>, ParseErr<'a>> {
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

    fn get_current_token(&mut self) -> ParseResult<'a, IndexedToken<'a>> {
        self.tokens
            .get(self.ind - 1)
            .ok_or(ParseErr::IndexingError)
            .copied()
    }
    fn peek_next_token(&self) -> Result<IndexedToken<'a>, ParseErr<'a>> {
        if let Some(token) = self.lookahead {
            Ok(token)
        } else {
            self.tokens
                .get(self.ind)
                .copied()
                .ok_or(ParseErr::UnexpectedEOF)
        }
    }

    fn peek_token_offset(&self, offset: usize) -> Result<IndexedToken<'a>, ParseErr<'a>> {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_annotation() {
        let mut parser = Parser::new(
            "@annotation1 @com.annotation2(val1, val2) @annotation3{key1: val1, key2: val2}",
        )
        .unwrap();

        assert_eq!(parser.annotation().unwrap(), "@annotation1");
        assert_eq!(parser.annotation().unwrap(), "@com.annotation2(val1, val2)");
        assert_eq!(
            parser.annotation().unwrap(),
            "@annotation3{key1: val1, key2: val2}"
        )
    }

    #[test]
    fn test_modifiers() {
        let mut parser = Parser::new("public static abstract").unwrap();

        assert_eq!(
            parser.modifiers().unwrap(),
            Modifiers {
                modifiers: vec!["public", "static", "abstract"],
                access_modifier: AccessModifier::Public
            }
        );
    }
}
