use super::super::{parser::Parser, token::Token::*};
use crate::types::*;

impl<'a> Parser<'a> {
    /// ```
    /// <arg_list> ::=
    /// "(" <annotations> ["final"] <ref_type> (
    ///     |   ("..." IDENTIFIER)
    ///     |   (IDENTIFIER {"[]"} {"," <annotations> ["final"] <ref_type> IDENTIFIER {"[]"}}
    ///         ["," "final" <annotations> <ref_type> "..." IDENTIFIER])
    /// ) ")"
    /// ```
    pub(crate) fn arg_list(&mut self) -> ParseResult<'a, Vec<RefType>> {
        let ctx = ("arg_list", self.peek_next_token().addr);
        // "("
        if self.get_next_token().token != LParen {
            return Err(ParseErrType::UnexpectedToken {
                expected: "LParen",
                got: vec![self.get_current_token().token],
            }
            .to_stack_parse_err(self.get_current_token().addr, ctx));
        }
        let mut v: Vec<RefType> = vec![];
        if self.peek_next_token().token == RParen {
            self.get_next_token();
            Ok(v)
        } else {
            // <annotations> ["final"]
            self.annotations().push_context(ctx)?;
            if self.peek_next_token().token == Keyword("final") {
                self.get_next_token();
            }

            // <reftype>
            v.push(self.ref_type().push_context(ctx)?);

            // | ("..." IDENTIFIER ")")
            if self.peek_next_token().token == Op("...") {
                self.get_next_token();
                consume_token!(self, ctx, Identifier(_), "Identifier");
                consume_token!(self, ctx, RParen, "RParen");
                return Ok(v);
            }

            // consume the Identifier
            let Identifier(_) = self.get_next_token().token else {
                return Err(ParseErrType::UnexpectedToken {
                    expected: "IDENTIFIER",
                    got: vec![self.get_current_token().token],
                }
                .to_stack_parse_err(self.get_current_token().addr, ctx));
            };

            // {"[]"}
            while self.peek_next_token().token == LBracket {
                self.get_next_token();
                consume_token!(self, ctx, RBracket, "RBracket");
                v.last_mut().unwrap().arr_dim += 1;
            }

            // {
            //      "," <annotations> ["final"] <ref_type> IDENTIFIER {"[]"}}
            //      ["," "final" <annotations> <ref_type> "..." IDENTIFIER])
            //  }
            while self.peek_next_token().token == Comma {
                // alright sorry for the many comments, I am not keeping track of things rn

                // ","
                self.get_next_token();

                // <annotations>
                self.annotations().push_context(ctx)?;
                if self.peek_next_token().token == Keyword("final") {
                    self.get_next_token();
                }

                // <type> before ID
                v.push(self.ref_type().push_context(ctx)?);

                // Escape to the
                // ```
                // "..." IDENTIFIER)
                // ```
                if self.peek_next_token().token == Op("...") {
                    // ...
                    self.get_next_token();
                    consume_token!(self, ctx, Identifier(_), "IDENTIFIER");
                    consume_token!(self, ctx, RParen, "RParen");
                    return Ok(v);
                }

                // not the "..." branch starts here
                consume_token!(self, ctx, Identifier(_), "IDENTIFIER");
                while self.peek_next_token().token == LBracket {
                    self.get_next_token();
                    consume_token!(self, ctx, RBracket, "RBracket");
                    v.last_mut().unwrap().arr_dim += 1;
                }
            }

            if self.get_next_token().token != RParen {
                return Err(ParseErrType::UnexpectedToken {
                    expected: "RParen",
                    got: vec![self.get_current_token().token],
                }
                .to_stack_parse_err(self.get_current_token().addr, ctx));
            }

            Ok(v)
        }
    }

    /// `<voidable_type> ::= "void" | <ref_type>`
    pub(crate) fn voidable_type(&mut self) -> ParseResult<'a, VoidableType> {
        let ctx = ("voidable_type", self.peek_next_token().addr);
        if self.peek_next_token().token == Keyword("void") {
            self.get_next_token();
            Ok(VoidableType::Void)
        } else {
            Ok(VoidableType::RefType(self.ref_type().push_context(ctx)?))
        }
    }

    /// `<ref_type> ::= <annotations> <qualified_name> <type_arg_lst> { "[]" }`
    pub(crate) fn ref_type(&mut self) -> ParseResult<'a, RefType> {
        let ctx = ("ref_type", self.peek_next_token().addr);
        // <qualified_name>
        self.annotations().push_context(ctx)?;
        let name: QualifiedName = self.qualified_name().push_context(ctx)?;

        // <type_arg_lst>
        let type_arg_list = self.type_arg_list().push_context(ctx)?;

        // { "[]" }
        let mut arr_dim: u8 = 0;
        while self.peek_next_token().token == LBracket {
            self.get_next_token();
            consume_token!(self, ctx, RBracket, "RBracket");
            arr_dim += 1;
        }

        Ok(RefType {
            name,
            type_arg_list,
            arr_dim,
        })
    }

    /// `<type_arg_list> ::= "<" <type_arg> { "," <type_arg> } ">"`
    pub(crate) fn type_arg_list(&mut self) -> ParseResult<'a, TypeArgList> {
        let ctx = ("type_arg_list", self.peek_next_token().addr);
        // [ "<" ...
        if self.peek_next_token().token == LessThan {
            self.get_next_token();
        } else {
            return Ok(TypeArgList(vec![]));
        };

        // <type_arg>
        let mut type_arg_list = TypeArgList(vec![self.type_arg().push_context(ctx)?]);

        // {"," <type_arg>}
        while self.peek_next_token().token == Comma {
            self.get_next_token();
            type_arg_list.0.push(self.type_arg().push_context(ctx)?);
        }

        // ... ">" ]
        self.consume_gt()?;

        Ok(type_arg_list)
    }

    /// `<type_arg> ::= (<ref_type> | "?" [ ( "extends" | "super" ) <ref_type> ]`
    pub(crate) fn type_arg(&mut self) -> ParseResult<'a, TypeArg> {
        let ctx = ("type_arg", self.peek_next_token().addr);
        self.annotations().push_context(ctx)?;
        if self.peek_next_token().token == QuestionMark {
            self.get_next_token();
            let token = self.peek_next_token();
            if token.token == Keyword("super") {
                self.get_next_token();
                return Ok(TypeArg::Super(self.ref_type().push_context(ctx)?));
            } else if token.token == Keyword("extends") {
                self.get_next_token();
                return Ok(TypeArg::Extends(self.ref_type().push_context(ctx)?));
            } else {
                return Ok(TypeArg::Wildcard);
            }
        } else {
            return Ok(TypeArg::Is(self.ref_type().push_context(ctx)?));
        }
    }

    /// `<type_param_list> ::= ["<" <type_param> { "," <type_param> } ">"]`
    pub(crate) fn type_param_list(&mut self) -> ParseResult<'a, TypeParamList> {
        let ctx = ("type_param_list", self.peek_next_token().addr);
        let mut list = TypeParamList(vec![]);

        if self.peek_next_token().token == LessThan {
            self.get_next_token();
            list.0.push(self.type_param().push_context(ctx)?);

            while self.peek_next_token().token == Comma {
                self.get_next_token();
                list.0.push(self.type_param().push_context(ctx)?);
            }

            self.consume_gt()?;
        }

        Ok(list)
    }

    /// `<type_param> ::= <annotations> IDENTIFIER ["extends" <ref_type> { "&" <ref_type> }]`
    pub(crate) fn type_param(&mut self) -> ParseResult<'a, TypeParam> {
        let ctx = ("type_param", self.peek_next_token().addr);
        self.annotations().push_context(ctx)?;
        let name = match self.get_next_token().token {
            Identifier(s) => s,
            token => {
                return Err(ParseErrType::UnexpectedToken {
                    expected: "IDENTIFIER",
                    got: vec![token],
                }
                .to_stack_parse_err(self.get_current_token().addr, ctx));
            }
        };

        if self.peek_next_token().token == Keyword("extends") {
            self.get_next_token();
            let mut extends_from = vec![self.ref_type().push_context(ctx)?];

            while self.peek_next_token().token == Op("&") {
                self.get_next_token();
                extends_from.push(self.ref_type().push_context(ctx)?);
            }

            Ok(TypeParam {
                name: name.to_owned(),
                extends_from,
            })
        } else {
            Ok(TypeParam {
                name: name.to_owned(),
                extends_from: vec![],
            })
        }
    }

    /// `<qualified_name> ::= IDENTIFIER {"." IDENTIFIER}`
    pub(crate) fn qualified_name(&mut self) -> ParseResult<'a, QualifiedName> {
        let ctx = ("qualified_name", self.peek_next_token().addr);
        let mut name = QualifiedName(vec![]);

        // IDENTIFIER
        let token = self.get_next_token();
        if let Identifier(s) = token.token {
            name.0.push(s.to_owned());
        } else {
            return Err(ParseErrType::UnexpectedToken {
                expected: "IDENTIFIER",
                got: vec![token.token],
            }
            .to_stack_parse_err(token.addr, ctx));
        }

        // {"." IDENTIFIER}
        loop {
            match (
                self.peek_token_offset(0).token,
                self.peek_token_offset(1).token,
            ) {
                (Dot, Identifier(s)) => {
                    name.0.push(s.to_owned());
                    self.get_next_token();
                    self.get_next_token();
                }
                _ => break,
            }
        }

        Ok(name)
    }

    /// `<annotations> ::= {<annotations>}`
    pub(crate) fn annotations(&mut self) -> ParseResult<'a, Vec<Annotation>> {
        let ctx = ("annotations", self.peek_next_token().addr);
        let mut v: Vec<Annotation> = vec![];
        while self.peek_next_token().token == At
            && self.peek_token_offset(1).token != Keyword("interface")
        {
            v.push(self.annotation().push_context(ctx)?);
        }
        Ok(v)
    }

    /// `<annotation> ::= "@" <qualified_name> [( "(" <skip_parens> ")" )| ( "{" <skip_brace> "}"
    /// )]`
    pub(crate) fn annotation(&mut self) -> ParseResult<'a, Annotation> {
        let ctx = ("annotation", self.peek_next_token().addr);
        if self.get_next_token().token != At {
            return Err(ParseErrType::UnexpectedToken {
                expected: "@",
                got: vec![self.get_current_token().token],
            }
            .to_stack_parse_err(self.get_current_token().addr, ctx));
        }
        if self.peek_next_token().token == Keyword("interface") {
            return Err(ParseErrType::UnexpectedToken {
                expected: "IDENTIFIER",
                got: vec![Keyword("interface")],
            }
            .to_stack_parse_err(self.get_current_token().addr, ctx));
        }

        let start_ind = self.get_current_token().addr;

        // <qualified_name>
        let name = self.qualified_name().push_context(ctx)?;

        // [( "(" <skip_parens> ")" )| ( "{" <skip_brace> "}" )]
        match self.peek_next_token().token {
            LBrace => {
                self.get_next_token();
                let mut stack = 1;
                while stack > 0 {
                    match self.get_next_token().token {
                        LBrace => stack += 1,
                        RBrace => stack -= 1,
                        EOF => {
                            return Err(ParseErrType::UnexpectedEOF
                                .to_stack_parse_err(self.peek_next_token().addr, ctx));
                        }
                        _ => {}
                    }
                }
            }
            LParen => {
                self.get_next_token();
                let mut stack = 1;
                while stack > 0 {
                    match self.get_next_token().token {
                        LParen => stack += 1,
                        RParen => stack -= 1,
                        EOF => {
                            return Err(ParseErrType::UnexpectedEOF
                                .to_stack_parse_err(self.peek_next_token().addr, ctx));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        let len = (self.get_current_token().addr + self.get_current_token().len) - start_ind;
        return Ok(Annotation {
            name,
            s: self.string[start_ind..start_ind + len].to_owned(),
        });
    }

    /// `<modifiers> ::= { "public" | "private" | "protected" | "abstract" | "static" | "final" |
    /// "strictfp" | "synchronized" | "native" | "transient" | "volatile" | "default" | "sealed" |
    /// "non-sealed" }`
    pub fn modifiers(&mut self) -> ParseResult<'a, Modifiers> {
        let ctx = ("modifiers", self.peek_next_token().addr);

        let mut modifiers = Modifiers {
            modifiers: vec![],
            access_modifier: AccessModifier::Default,
        };

        while let Keyword(s) = self.peek_next_token().token {
            if matches!(s, "public" | "private" | "protected") {
                if modifiers.access_modifier != AccessModifier::Default {
                    return Err(ParseErrType::UnexpectedToken {
                        expected: "non-access modifier",
                        got: vec![Keyword(s)],
                    }
                    .to_stack_parse_err(self.peek_next_token().addr, ctx));
                }
                modifiers.access_modifier = match s {
                    "public" => AccessModifier::Public,
                    "private" => AccessModifier::Private,
                    "protected" => AccessModifier::Protected,
                    _ => unreachable!("we have guaranteed public/private/protected"),
                };
            } else if s == "non" {
                if self.peek_token_offset(1).token != Op("-") {
                    break;
                }
                if self.peek_token_offset(2).token != Keyword("sealed") {
                    break;
                }
                modifiers.modifiers.push("non-sealed".to_owned());
                self.get_next_token();
                self.get_next_token();
                self.get_next_token();
            } else if !matches!(
                s,
                "abstract"
                    | "static"
                    | "final"
                    | "strictfp"
                    | "synchronized"
                    | "native"
                    | "transient"
                    | "volatile"
                    | "default"
            ) {
                break;
            }
            self.get_next_token();
            modifiers.modifiers.push(s.to_owned());
        }

        Ok(modifiers)
    }
}

//-----------------------------------------------------------------
//--------------------------- UNIT TEST ---------------------------
//-----------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_annotation() {
        let mut parser = Parser::new(
            "@annotation1 @com.annotation2(val1, val2) @annotation3{key1: val1, key2: val2}",
        )
        .unwrap();

        assert_eq!(
            parser.annotation().unwrap(),
            Annotation {
                name: QualifiedName(vec!["annotation1".to_owned()]),
                s: "@annotation1".to_owned()
            }
        );
        assert_eq!(
            parser.annotation().unwrap(),
            Annotation {
                name: QualifiedName(vec!["com".to_owned(), "annotation2".to_owned()]),
                s: "@com.annotation2(val1, val2)".to_owned()
            },
        );
        assert_eq!(
            parser.annotation().unwrap(),
            Annotation {
                name: QualifiedName(vec!["annotation3".to_owned()]),
                s: "@annotation3{key1: val1, key2: val2}".to_owned()
            }
        )
    }

    #[test]
    fn test_modifiers() {
        let mut parser = Parser::new("public static abstract").unwrap();

        assert_eq!(
            parser.modifiers().unwrap(),
            Modifiers {
                modifiers: vec![
                    "public".to_owned(),
                    "static".to_owned(),
                    "abstract".to_owned()
                ],
                access_modifier: AccessModifier::Public
            }
        );
    }

    #[test]
    fn test_ref_type() {
        fn test_ref_type_str(s: &str, r: RefType) {
            let mut parser = Parser::new(s).unwrap();
            assert_eq!(parser.ref_type().unwrap(), r);
        }

        test_ref_type_str(
            "util.Map<@NotNull String, @NotNull ? extends @NotNull int[][], ? super @NotNull Array<Integer>>[][][]",
            RefType {
                name: QualifiedName(vec!["util".to_owned(), "Map".to_owned()]),
                type_arg_list: TypeArgList(vec![
                    TypeArg::Is(RefType {
                        name: QualifiedName(vec!["String".to_owned()]),
                        type_arg_list: TypeArgList(vec![]),
                        arr_dim: 0,
                    }),
                    TypeArg::Extends(RefType {
                        name: QualifiedName(vec!["int".to_owned()]),
                        type_arg_list: TypeArgList(vec![]),
                        arr_dim: 2,
                    }),
                    TypeArg::Super(RefType {
                        name: QualifiedName(vec!["Array".to_owned()]),
                        type_arg_list: TypeArgList(vec![TypeArg::Is(RefType {
                            name: QualifiedName(vec!["Integer".to_owned()]),
                            type_arg_list: TypeArgList(vec![]),
                            arr_dim: 0,
                        })]),
                        arr_dim: 0,
                    }),
                ]),
                arr_dim: 3,
            },
        );
    }

    #[test]
    fn test_type_param_list() {
        let mut parser = Parser::new(
            "<@NotNull @Anno K extends Comparable<K> & com.util.Node, V extends Vector<Token>>",
        )
        .unwrap();
        assert_eq!(
            parser.type_param_list().unwrap(),
            TypeParamList(vec![
                TypeParam {
                    name: "K".to_owned(),
                    extends_from: vec![
                        RefType {
                            name: QualifiedName(vec!["Comparable".to_owned()]),
                            type_arg_list: TypeArgList(vec![TypeArg::Is(RefType {
                                name: QualifiedName(vec!["K".to_owned()]),
                                type_arg_list: TypeArgList(vec![]),
                                arr_dim: 0
                            })]),
                            arr_dim: 0,
                        },
                        RefType {
                            name: QualifiedName(vec![
                                "com".to_owned(),
                                "util".to_owned(),
                                "Node".to_owned()
                            ]),
                            type_arg_list: TypeArgList(vec![]),
                            arr_dim: 0,
                        }
                    ]
                },
                TypeParam {
                    name: "V".to_owned(),
                    extends_from: vec![RefType {
                        name: QualifiedName(vec!["Vector".to_owned()]),
                        type_arg_list: TypeArgList(vec![TypeArg::Is(RefType {
                            name: QualifiedName(vec!["Token".to_owned()]),
                            type_arg_list: TypeArgList(vec![]),
                            arr_dim: 0
                        })]),
                        arr_dim: 0,
                    }],
                }
            ])
        );
    }

    #[test]
    fn test_arg_list() {
        let mut parser = Parser::new("(final ArrayList<Integer> lst, final Integer... b)").unwrap();
        assert_eq!(
            parser.arg_list().unwrap(),
            vec![
                RefType {
                    name: QualifiedName(vec!["ArrayList".to_owned()]),
                    type_arg_list: TypeArgList(vec![TypeArg::Is(RefType {
                        name: QualifiedName(vec!["Integer".to_owned()]),
                        type_arg_list: TypeArgList(vec![]),
                        arr_dim: 0
                    })]),
                    arr_dim: 0,
                },
                RefType {
                    name: QualifiedName(vec!["Integer".to_owned()]),
                    type_arg_list: TypeArgList(vec![]),
                    arr_dim: 0,
                }
            ]
        );
        parser = Parser::new("(String... val)").unwrap();
        assert_eq!(
            parser.arg_list().unwrap(),
            vec![RefType {
                name: QualifiedName(vec!["String".to_owned()]),
                type_arg_list: TypeArgList(vec![]),
                arr_dim: 0,
            }]
        );
        parser = Parser::new("(int a, char b)").unwrap();
        assert_eq!(
            parser.arg_list().unwrap(),
            vec![
                RefType {
                    name: QualifiedName(vec!["int".to_owned()]),
                    type_arg_list: TypeArgList(vec![]),
                    arr_dim: 0,
                },
                RefType {
                    name: QualifiedName(vec!["char".to_owned()]),
                    type_arg_list: TypeArgList(vec![]),
                    arr_dim: 0,
                },
            ]
        );
    }
}
