use crate::parser::{
    self,
    lexer::Lexer,
    token::{
        IndexedToken,
        Token::{self, *},
    },
    types::{
        AccessModifier, Annotation, ImportObject, JavaFile, Member, MemberKind, Modifiers,
        ParseErr, ParseResult, QualifiedName, RefType, Type, TypeArg, TypeArgList, TypeBody,
        TypeKind, TypeParam, TypeParamList, VoidableType,
    },
};
pub struct Parser<'a> {
    string: &'a str,
    tokens: Vec<IndexedToken<'a>>,
    ind: usize,
    lookahead: Option<IndexedToken<'a>>,
}

// ---------------------------------------------------------------------
// --------------------- Parsing ---------------------------------------
// ---------------------------------------------------------------------

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
    // ----------------------- Class Nonterminals --------------------------
    // ---------------------------------------------------------------------

    /// `<class_decl> ::= "class" IDENTIFIER <type_param_list> [ "extends" <ref_type> ]
    /// [ "implements" <ref_type> { "," <ref_type> } ] <class_body>
    fn class_decl(&mut self, prefix: QualifiedName<'a>) -> ParseResult<'a, Type<'a>> {
        // "class"
        if self.get_next_token()?.token != Keyword("class") {
            return Err(ParseErr::UnexpectedToken {
                expected: "class",
                got: vec![self.get_current_token()?.token],
            });
        }

        // IDENTIFIER
        let mut name = QualifiedName(prefix.0.clone());
        name.0.push(match self.get_next_token()?.token {
            Identifier(s) => s,
            token => {
                return Err(ParseErr::UnexpectedToken {
                    expected: "IDENTIFIER",
                    got: vec![token],
                });
            }
        });

        // <type_param> (unimportant for now)
        self.type_param_list()?;

        // ["extends" <ref_type>]
        let inherits_from: Option<RefType<'a>> =
            if self.peek_next_token()?.token == Keyword("extends") {
                self.get_next_token()?;
                Some(self.ref_type()?)
            } else {
                None
            };

        // ["implements" <ref_type> {"," <ref_type>}]
        let implement_interfaces: Vec<RefType<'a>> =
            if self.peek_next_token()?.token == Keyword("implements") {
                self.get_next_token()?;
                let mut vector = vec![self.ref_type()?];
                while self.peek_next_token()?.token == Comma {
                    self.get_next_token()?;
                    vector.push(self.ref_type()?)
                }
                vector
            } else {
                vec![]
            };

        let type_kind = TypeKind::Class {
            inherits_from,
            implement_interfaces,
        };

        // <class_body>
        let body = self.class_body(name.clone())?;

        // use default modifiers and annotation
        let typeclass = Type {
            name,
            modifiers: Modifiers {
                modifiers: vec![],
                access_modifier: AccessModifier::Default,
            },
            type_kind,
            body,
            annotation: vec![],
        };

        Ok(typeclass)
    }

    /// `<class_body>      ::= "{" {<member_decl>} "}"`, where
    /// `<member_decl>     ::= {<annotation>} <modifiers> ( <method_decl> | <property_decl> | <type_decl> )`
    fn class_body(&mut self, prefix: QualifiedName<'a>) -> ParseResult<'a, TypeBody<'a>> {
        if self.get_next_token()?.token != LBrace {
            return Err(ParseErr::UnexpectedToken {
                expected: "LBrace",
                got: vec![self.get_current_token()?.token],
            });
        }

        // if the next token is not closing the body, then it must be still
        // a member.
        let mut body = TypeBody {
            members: vec![],
            subtypes: vec![],
        };

        // {<member_decl>}, inside is the <member_decl>
        while self.peek_next_token()?.token != RBrace {
            if self.peek_next_token()?.token == EOF {
                return Err(ParseErr::UnexpectedEOF);
            }

            let annotations: Vec<Annotation> = self.annotations()?;
            let modifiers = self.modifiers()?;

            match (
                self.peek_next_token()?.token,
                self.peek_token_offset(1)?.token,
            ) {
                // initializer block
                (LBrace, _) => {
                    self.skip_brace(LBrace, RBrace)?;
                }
                // Types: class
                (Keyword("class"), _) => {
                    let mut typeclass = self.class_decl(prefix.clone())?;
                    typeclass.modifiers = modifiers;
                    typeclass.annotation = annotations;
                    body.subtypes.push(typeclass);
                }
                // Types: enum
                (Keyword("enum"), _) => {
                    let mut typeclass = self.enum_decl(prefix.clone())?;
                    typeclass.modifiers = modifiers;
                    typeclass.annotation = annotations;
                    body.subtypes.push(typeclass);
                }
                // Types: annotation
                (At, Keyword("interface")) => {
                    let mut typeclass = self.annotation_decl(prefix.clone())?;
                    typeclass.modifiers = modifiers;
                    typeclass.annotation = annotations;
                    body.subtypes.push(typeclass);
                }

                // Types: interface
                (Keyword("interface"), _) => {
                    let mut typeclass = self.enum_decl(prefix.clone())?;
                    typeclass.modifiers = modifiers;
                    typeclass.annotation = annotations;
                    body.subtypes.push(typeclass);
                }
                // Members: method with type_param
                (LessThan, _) => {
                    // <type_param_list> <voidable_type> IDENTIFIER <arg_list> <method_body>
                    let type_param_list = self.type_param_list()?;
                    let output = self.voidable_type()?;
                    let name = if let Identifier(s) = self.get_next_token()?.token {
                        s
                    } else {
                        return Err(ParseErr::UnexpectedToken {
                            expected: "IDENTIFIER",
                            got: vec![self.get_current_token()?.token],
                        });
                    };
                    let input = self.arg_list()?;
                    let throws = if self.peek_next_token()?.token == Keyword("throws") {
                        let mut v: Vec<RefType<'a>> = vec![];

                        // "throws" <ref_type>
                        self.get_next_token()?;
                        v.push(self.ref_type()?);

                        // {"," <ref_type>}
                        while self.peek_next_token()?.token == Comma {
                            self.get_next_token()?;
                            v.push(self.ref_type()?);
                        }

                        v
                    } else {
                        vec![]
                    };
                    self.skip_brace(LBrace, RBrace)?;
                    body.members.push(Member {
                        name,
                        member_kind: MemberKind::Method {
                            type_param_list,
                            input,
                            output,
                            throws,
                        },
                        annotations,
                        modifiers,
                    })
                }
                // Members: either property or method
                (Keyword("void"), _) | (Identifier(_), _) | (At, _) => {
                    // <ref_type> <annotations> IDENTIFIER (
                    //  | <arg_list> <method_body>
                    //  | [<assignment>] {"," IDENTIFIER [<assignment>]} ";"
                    // )
                    let output = self.voidable_type()?;
                    let reftype = if let VoidableType::RefType(s) = output.clone() {
                        Ok(s)
                    } else {
                        Err(ParseErr::UnexpectedToken {
                            expected: "IDENTIFIER",
                            got: vec![Keyword("void")],
                        })
                    };
                    let name = if let Identifier(s) = self.get_next_token()?.token {
                        s
                    } else {
                        return Err(ParseErr::UnexpectedToken {
                            expected: "IDENTIFIER",
                            got: vec![self.get_current_token()?.token],
                        });
                    };
                    match self.peek_next_token()?.token {
                        LParen => {
                            let input = self.arg_list()?;
                            let throws = if self.peek_next_token()?.token == Keyword("throws") {
                                let mut v: Vec<RefType<'a>> = vec![];

                                // "throws" <ref_type>
                                self.get_next_token()?;
                                v.push(self.ref_type()?);

                                // {"," <ref_type>}
                                while self.peek_next_token()?.token == Comma {
                                    self.get_next_token()?;
                                    v.push(self.ref_type()?);
                                }

                                v
                            } else {
                                vec![]
                            };
                            if self.peek_next_token()?.token == Semicolon {
                                self.get_next_token()?;
                            } else if self.peek_next_token()?.token == LBrace {
                                self.skip_brace(LBrace, RBrace)?;
                            } else {
                                return Err(ParseErr::UnexpectedToken {
                                    expected: "Semicolon | LBrace",
                                    got: vec![self.peek_next_token()?.token],
                                });
                            }
                            body.members.push(Member {
                                name,
                                member_kind: MemberKind::Method {
                                    type_param_list: TypeParamList(vec![]),
                                    input,
                                    output,
                                    throws,
                                },
                                annotations,
                                modifiers,
                            });
                        }
                        Assignment("=") | Comma | Semicolon => {
                            // resolve assignment
                            // = <skip_assignment> {"," IDENTIFIER ["=" <skip_assignment>]} ";"

                            // "=" <skip_assignment>
                            if self.peek_next_token()?.token == Assignment("=") {
                                loop {
                                    match self.peek_next_token()?.token {
                                        LBrace => self.skip_brace(LBrace, RBrace)?,
                                        LParen => self.skip_brace(LParen, RParen)?,
                                        LBracket => self.skip_brace(LBracket, RBracket)?,
                                        Semicolon => {
                                            self.get_next_token()?;
                                            break;
                                        }
                                        Comma if self.check_end_assignment_comma()? => {
                                            break;
                                        }
                                        _ => {
                                            self.get_next_token()?;
                                        }
                                    };
                                }
                            }

                            body.members.push(Member {
                                name,
                                member_kind: MemberKind::Property {
                                    reftype: reftype.clone()?,
                                },
                                annotations: annotations.clone(),
                                modifiers: modifiers.clone(),
                            });

                            // {"," IDENTIFIER ["=" <skip_assignment>]} ";"
                            while self.get_next_token()?.token == Comma {
                                // IDENTIFIER
                                let Identifier(name) = self.get_next_token()?.token else {
                                    return Err(ParseErr::UnexpectedToken {
                                        expected: "IDENTIFIER",
                                        got: vec![self.get_current_token()?.token],
                                    });
                                };

                                match self.peek_next_token()?.token {
                                    Assignment("=") => loop {
                                        match self.peek_next_token()?.token {
                                            LBrace => self.skip_brace(LBrace, RBrace)?,
                                            LParen => self.skip_brace(LParen, RParen)?,
                                            LBracket => self.skip_brace(LBracket, RBracket)?,
                                            Semicolon => break,
                                            Comma if self.check_end_assignment_comma()? => break,
                                            _ => {
                                                self.get_next_token()?;
                                            }
                                        };
                                    },
                                    Semicolon | Comma => {}
                                    token => {
                                        return Err(ParseErr::UnexpectedToken {
                                            expected: "Assignment | Semicolon | Comma",
                                            got: vec![token],
                                        });
                                    }
                                };

                                body.members.push(Member {
                                    name,
                                    member_kind: MemberKind::Property {
                                        reftype: reftype.clone()?,
                                    },
                                    annotations: annotations.clone(),
                                    modifiers: modifiers.clone(),
                                });
                            }
                        }
                        token => {
                            return Err(ParseErr::UnexpectedToken {
                                expected: "LBrace | = | Comma",
                                got: vec![token],
                            });
                        }
                    }
                }
                // error
                (token1, _) => {
                    return Err(ParseErr::UnexpectedToken {
                        expected: "type_decl | type_param",
                        got: vec![token1],
                    });
                }
            };
        }
        // consume the RBrace
        self.get_next_token()?;
        Ok(body)
    }

    // ---------------------------------------------------------------------
    // ----------------------- Enum Nonterminals ---------------------------
    // ---------------------------------------------------------------------

    fn enum_decl(&mut self, prefix: QualifiedName<'a>) -> ParseResult<'a, Type<'a>> {
        Err(ParseErr::UnimplementedError)
    }

    // ---------------------------------------------------------------------
    // ----------------------- Interface Nonterminals ----------------------
    // ---------------------------------------------------------------------
    fn interface_decl(&mut self, prefix: QualifiedName<'a>) -> ParseResult<'a, Type<'a>> {
        Err(ParseErr::UnimplementedError)
    }

    // ---------------------------------------------------------------------
    // ----------------------- Annotation Nonterminals ---------------------
    // ---------------------------------------------------------------------

    fn annotation_decl(&mut self, prefix: QualifiedName<'a>) -> ParseResult<'a, Type<'a>> {
        Err(ParseErr::UnimplementedError)
    }
}

// ---------------------------------------------------------------------
// ------------------- Util nonterms -----------------------------------
// ---------------------------------------------------------------------

impl<'a> Parser<'a> {
    /// ```
    /// <arg_list> ::=
    /// "(" <annotations> ["final"] <ref_type> (
    ///     |   ("..." IDENTIFIER)
    ///     |   (IDENTIFIER {"[]"} {"," <annotations> ["final"] <ref_type> IDENTIFIER {"[]"}}
    ///         ["," "final" <annotations> <ref_type> "..." IDENTIFIER])
    /// ) ")"
    /// ```
    fn arg_list(&mut self) -> ParseResult<'a, Vec<RefType<'a>>> {
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
    fn voidable_type(&mut self) -> ParseResult<'a, VoidableType<'a>> {
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
    fn ref_type(&mut self) -> ParseResult<'a, RefType<'a>> {
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
    fn type_arg_list(&mut self) -> ParseResult<'a, TypeArgList<'a>> {
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
    fn type_arg(&mut self) -> ParseResult<'a, TypeArg<'a>> {
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
    fn type_param_list(&mut self) -> ParseResult<'a, TypeParamList<'a>> {
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
    fn type_param(&mut self) -> ParseResult<'a, TypeParam<'a>> {
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

    /// `<annotations> ::= {<annotations>}`
    fn annotations(&mut self) -> ParseResult<'a, Vec<Annotation<'a>>> {
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
    fn annotation(&mut self) -> ParseResult<'a, Annotation<'a>> {
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

// ---------------------------------------------------------------------
// ------------------------ helpers for Parser -------------------------
// ---------------------------------------------------------------------

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

    fn consume_gt(&mut self) -> ParseResult<'a, ()> {
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

    fn skip_brace(&mut self, open_brace: Token, close_brace: Token) -> ParseResult<'a, ()> {
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
    fn check_end_assignment_comma(&mut self) -> ParseResult<'a, bool> {
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
    fn skip_annotations(&self, offset: usize) -> ParseResult<'a, usize> {
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
    fn skip_brace_peek_forward(
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
// ------------------------ TESTS --------------------------------------
// ---------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_class_decl() {
        let mut parser = Parser::new(
            "@NotNull public class MyClass<T extends Comparable<T>> extends MyParentClass<T> implements Printable, GetTAble { 
                @NotNull @JsonIgnore
                java.util.HashMap<String, Integer> a, 
                    b = new HashMap<String, Integer>(), 
                    c; 

                @Nullable
                float fa = 1.0f, fb = math.PI / 6, fc = fb * 7;

                @NotNull
                float fe, fh = fa;

                @NotNull
                float fg;
                
                @Getter
                public Integer fromA(@NotNull String key) { 
                    return this.a.get(key); 
                }
                
                @Getter
                public <T> T getT(String key, java.util.HashMap<Integer, T> hashmap) {
                    return hashmap.get(this.a.get(key));
                }
                
                abstract public int joinAbc();
            }",
        )
        .unwrap();
        let res = parser.type_decl(QualifiedName(vec![]));
        println!("res:\n {:#?}", res);
    }

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
