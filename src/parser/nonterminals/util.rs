use super::super::{parser::Parser, token::Token::*, types::*};

impl<'a> Parser<'a> {
    /// ```
    /// <arg_list> ::=
    /// "(" <annotations> ["final"] <ref_type> (
    ///     |   ("..." IDENTIFIER)
    ///     |   (IDENTIFIER {"[]"} {"," <annotations> ["final"] <ref_type> IDENTIFIER {"[]"}}
    ///         ["," "final" <annotations> <ref_type> "..." IDENTIFIER])
    /// ) ")"
    /// ```
    pub(crate) fn arg_list(&mut self) -> ParseResult<'a, Vec<RefType<'a>>> {
        // "("
        if self.get_next_token()?.token != LParen {
            return Err(ParseErr::UnexpectedToken {
                expected: "LParen",
                got: vec![self.get_current_token()?.token],
            });
        }
        let mut v: Vec<RefType> = vec![];
        if self.peek_next_token()?.token == RParen {
            self.get_next_token()?;
            Ok(v)
        } else {
            // <annotations> ["final"]
            self.annotations()?;
            if self.peek_next_token()?.token == Keyword("final") {
                self.get_next_token()?;
            }

            // <reftype>
            v.push(self.ref_type()?);

            // | ("..." IDENTIFIER ")")
            if self.peek_next_token()?.token == Op("...") {
                self.get_next_token()?;
                let Identifier(_) = self.get_next_token()?.token else {
                    return Err(ParseErr::UnexpectedToken {
                        expected: "IDENTIFIER",
                        got: vec![self.get_current_token()?.token],
                    });
                };
                if self.get_next_token()?.token != RParen {
                    return Err(ParseErr::UnexpectedToken {
                        expected: "RParen",
                        got: vec![self.get_current_token()?.token],
                    });
                }
                return Ok(v);
            }

            // consume the Identifier
            let Identifier(_) = self.get_next_token()?.token else {
                return Err(ParseErr::UnexpectedToken {
                    expected: "IDENTIFIER",
                    got: vec![self.get_current_token()?.token],
                });
            };

            // {"[]"}
            while self.peek_next_token()?.token == LBracket {
                self.get_next_token()?;
                if self.get_next_token()?.token != RBracket {
                    return Err(ParseErr::UnexpectedToken {
                        expected: "RBracket",
                        got: vec![self.get_current_token()?.token],
                    });
                }
                v.last_mut().unwrap().arr_dim += 1;
            }

            while self.peek_next_token()?.token == Comma {
                self.get_next_token()?;
                self.annotations()?;
                if self.peek_next_token()?.token == Keyword("final") {
                    self.get_next_token()?;
                }

                v.push(self.ref_type()?);

                if self.peek_next_token()?.token == Op("...") {
                    self.get_next_token()?;
                    let Identifier(_) = self.get_next_token()?.token else {
                        return Err(ParseErr::UnexpectedToken {
                            expected: "IDENTIFIER",
                            got: vec![self.get_current_token()?.token],
                        });
                    };
                    if self.get_next_token()?.token != RParen {
                        return Err(ParseErr::UnexpectedToken {
                            expected: "RParen",
                            got: vec![self.get_current_token()?.token],
                        });
                    }
                    return Ok(v);
                }

                let Identifier(_) = self.get_next_token()?.token else {
                    return Err(ParseErr::UnexpectedToken {
                        expected: "IDENTIFIER",
                        got: vec![self.get_current_token()?.token],
                    });
                };

                while self.peek_next_token()?.token == LBracket {
                    self.get_next_token()?;
                    if self.get_next_token()?.token != RBracket {
                        return Err(ParseErr::UnexpectedToken {
                            expected: "RBracket",
                            got: vec![self.get_current_token()?.token],
                        });
                    }
                }
            }

            if self.get_next_token()?.token != RParen {
                return Err(ParseErr::UnexpectedToken {
                    expected: "RParen",
                    got: vec![self.get_current_token()?.token],
                });
            }

            Ok(v)
        }
    }
    /// `<voidable_type> ::= "void" | <ref_type>`
    pub(crate) fn voidable_type(&mut self) -> ParseResult<'a, VoidableType<'a>> {
        if let Ok(token) = self.peek_next_token()
            && token.token == Keyword("void")
        {
            self.get_next_token()?;
            Ok(VoidableType::Void)
        } else {
            Ok(VoidableType::RefType(self.ref_type()?))
        }
    }

    /// `<ref_type> ::= <annotations> <qualified_name> <type_arg_lst> { "[]" }`
    pub(crate) fn ref_type(&mut self) -> ParseResult<'a, RefType<'a>> {
        // <qualified_name>
        self.annotations()?;
        let name: QualifiedName<'a> = self.qualified_name()?;

        // <type_arg_lst>
        let type_arg_list = self.type_arg_list()?;

        // { "[]" }
        let mut arr_dim: u8 = 0;
        loop {
            if let Ok(token1) = self.peek_next_token() {
                if token1.token != LBracket {
                    break;
                }
                self.get_next_token().unwrap();
                if self.get_next_token()?.token != RBracket {
                    return Err(ParseErr::UnexpectedToken {
                        expected: "]",
                        got: vec![self.get_current_token().unwrap().token],
                    });
                }
                arr_dim += 1;
            } else {
                break;
            }
        }

        Ok(RefType {
            name,
            type_arg_list,
            arr_dim,
        })
    }

    /// `<type_arg_list> ::= "<" <type_arg> { "," <type_arg> } ">"`
    pub(crate) fn type_arg_list(&mut self) -> ParseResult<'a, TypeArgList<'a>> {
        // [ "<" ...
        if let Ok(token) = self.peek_next_token()
            && token.token == LessThan
        {
            self.get_next_token()?;
        } else {
            return Ok(TypeArgList(vec![]));
        };

        // <type_arg>
        let mut type_arg_list = TypeArgList(vec![self.type_arg()?]);

        // {"," <type_arg>}
        while let Ok(token) = self.peek_next_token()
            && token.token == Comma
        {
            self.get_next_token()?;
            type_arg_list.0.push(self.type_arg()?);
        }

        // ... ">" ]
        self.consume_gt()?;

        Ok(type_arg_list)
    }

    /// `<type_arg> ::= (<ref_type> | "?" [ ( "extends" | "super" ) <ref_type> ]`
    pub(crate) fn type_arg(&mut self) -> ParseResult<'a, TypeArg<'a>> {
        self.annotations()?;
        if self.peek_next_token()?.token == QuestionMark {
            self.get_next_token()?;
            if let Ok(token) = self.peek_next_token() {
                if token.token == Keyword("super") {
                    self.get_next_token()?;
                    return Ok(TypeArg::Super(self.ref_type()?));
                } else if token.token == Keyword("extends") {
                    self.get_next_token()?;
                    return Ok(TypeArg::Extends(self.ref_type()?));
                } else {
                    return Ok(TypeArg::Wildcard);
                }
            } else {
                return Ok(TypeArg::Wildcard);
            }
        } else {
            return Ok(TypeArg::Is(self.ref_type()?));
        }
    }

    /// `<type_param_list> ::= ["<" <type_param> { "," <type_param> } ">"]`
    pub(crate) fn type_param_list(&mut self) -> ParseResult<'a, TypeParamList<'a>> {
        let mut list = TypeParamList(vec![]);

        if let Ok(token) = self.peek_next_token()
            && token.token == LessThan
        {
            self.get_next_token()?;
            list.0.push(self.type_param()?);

            while let Ok(token) = self.peek_next_token()
                && token.token == Comma
            {
                self.get_next_token()?;
                list.0.push(self.type_param()?);
            }

            self.consume_gt()?;
        }

        Ok(list)
    }

    /// `<type_param> ::= <annotations> IDENTIFIER ["extends" <ref_type> { "&" <ref_type> }]`
    pub(crate) fn type_param(&mut self) -> ParseResult<'a, TypeParam<'a>> {
        self.annotations()?;
        let name = match self.get_next_token()?.token {
            Identifier(s) => s,
            token => {
                return Err(ParseErr::UnexpectedToken {
                    expected: "IDENTIFIER",
                    got: vec![token],
                });
            }
        };

        if let Ok(token) = self.peek_next_token()
            && token.token == Keyword("extends")
        {
            self.get_next_token()?;
            let mut extends_from = vec![self.ref_type()?];

            while let Ok(token) = self.peek_next_token()
                && token.token == Op("&")
            {
                self.get_next_token()?;
                extends_from.push(self.ref_type()?);
            }

            Ok(TypeParam { name, extends_from })
        } else {
            Ok(TypeParam {
                name,
                extends_from: vec![],
            })
        }
    }

    /// `<qualified_name> ::= IDENTIFIER {"." IDENTIFIER}`
    pub(crate) fn qualified_name(&mut self) -> ParseResult<'a, QualifiedName<'a>> {
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

    /// `<annotations> ::= {<annotations>}`
    pub(crate) fn annotations(&mut self) -> ParseResult<'a, Vec<Annotation<'a>>> {
        let mut v: Vec<Annotation<'a>> = vec![];
        while self.peek_next_token()?.token == At
            && self.peek_token_offset(1)?.token != Keyword("interface")
        {
            v.push(self.annotation()?);
        }
        Ok(v)
    }

    /// `<annotation> ::= "@" <qualified_name> [( "(" <skip_parens> ")" )| ( "{" <skip_brace> "}"
    /// )]`
    pub(crate) fn annotation(&mut self) -> ParseResult<'a, Annotation<'a>> {
        if self.get_next_token()?.token != At {
            return Err(ParseErr::UnexpectedToken {
                expected: "@",
                got: vec![self.get_current_token()?.token],
            });
        }
        if self.peek_next_token()?.token == Keyword("interface") {
            return Err(ParseErr::UnexpectedToken {
                expected: "IDENTIFIER",
                got: vec![Keyword("interface")],
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
        return Ok(Annotation(&self.string[start_ind..start_ind + len]));
    }

    /// `<modifiers> ::= { "public" | "private" | "protected" | "abstract" | "static" | "final" |
    /// "strictfp" | "synchronized" | "native" | "transient" | "volatile" | "default" | "sealed" |
    /// "non-sealed" }`
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
            } else if s == "non" {
                if self.peek_token_offset(1)?.token != Op("-") {
                    break;
                }
                if self.peek_token_offset(2)?.token != Keyword("sealed") {
                    break;
                }
                modifiers.modifiers.push("non-sealed");
                self.get_next_token()?;
                self.get_next_token()?;
                self.get_next_token()?;
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
            self.get_next_token()?;
            modifiers.modifiers.push(s);
        }

        Ok(modifiers)
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

        assert_eq!(parser.annotation().unwrap(), Annotation("@annotation1"));
        assert_eq!(
            parser.annotation().unwrap(),
            Annotation("@com.annotation2(val1, val2)")
        );
        assert_eq!(
            parser.annotation().unwrap(),
            Annotation("@annotation3{key1: val1, key2: val2}")
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

    #[test]
    fn test_ref_type() {
        fn test_ref_type_str(s: &str, r: RefType) {
            let mut parser = Parser::new(s).unwrap();
            assert_eq!(parser.ref_type().unwrap(), r);
        }

        test_ref_type_str(
            "util.Map<@NotNull String, @NotNull ? extends @NotNull int[][], ? super @NotNull Array<Integer>>[][][]",
            RefType {
                name: QualifiedName(vec!["util", "Map"]),
                type_arg_list: TypeArgList(vec![
                    TypeArg::Is(RefType {
                        name: QualifiedName(vec!["String"]),
                        type_arg_list: TypeArgList(vec![]),
                        arr_dim: 0,
                    }),
                    TypeArg::Extends(RefType {
                        name: QualifiedName(vec!["int"]),
                        type_arg_list: TypeArgList(vec![]),
                        arr_dim: 2,
                    }),
                    TypeArg::Super(RefType {
                        name: QualifiedName(vec!["Array"]),
                        type_arg_list: TypeArgList(vec![TypeArg::Is(RefType {
                            name: QualifiedName(vec!["Integer"]),
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
                    name: "K",
                    extends_from: vec![
                        RefType {
                            name: QualifiedName(vec!["Comparable"]),
                            type_arg_list: TypeArgList(vec![TypeArg::Is(RefType {
                                name: QualifiedName(vec!["K"]),
                                type_arg_list: TypeArgList(vec![]),
                                arr_dim: 0
                            })]),
                            arr_dim: 0,
                        },
                        RefType {
                            name: QualifiedName(vec!["com", "util", "Node"]),
                            type_arg_list: TypeArgList(vec![]),
                            arr_dim: 0,
                        }
                    ]
                },
                TypeParam {
                    name: "V",
                    extends_from: vec![RefType {
                        name: QualifiedName(vec!["Vector"]),
                        type_arg_list: TypeArgList(vec![TypeArg::Is(RefType {
                            name: QualifiedName(vec!["Token"]),
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
                    name: QualifiedName(vec!["ArrayList"]),
                    type_arg_list: TypeArgList(vec![TypeArg::Is(RefType {
                        name: QualifiedName(vec!["Integer"]),
                        type_arg_list: TypeArgList(vec![]),
                        arr_dim: 0
                    })]),
                    arr_dim: 0,
                },
                RefType {
                    name: QualifiedName(vec!["Integer"]),
                    type_arg_list: TypeArgList(vec![]),
                    arr_dim: 0,
                }
            ]
        );
        parser = Parser::new("(String... val)").unwrap();
        assert_eq!(
            parser.arg_list().unwrap(),
            vec![RefType {
                name: QualifiedName(vec!["String"]),
                type_arg_list: TypeArgList(vec![]),
                arr_dim: 0,
            }]
        );
        parser = Parser::new("(int a, char b)").unwrap();
        assert_eq!(
            parser.arg_list().unwrap(),
            vec![
                RefType {
                    name: QualifiedName(vec!["int"]),
                    type_arg_list: TypeArgList(vec![]),
                    arr_dim: 0,
                },
                RefType {
                    name: QualifiedName(vec!["char"]),
                    type_arg_list: TypeArgList(vec![]),
                    arr_dim: 0,
                },
            ]
        );
    }
}
