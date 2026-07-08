use super::super::{parser::Parser, token::Token::*};
use crate::types::*;

// ---------------------------------------------------------------------
// ----------------------- Annotation Nonterminals ---------------------
// ---------------------------------------------------------------------
impl<'a> Parser<'a> {
    pub(crate) fn annotation_decl(
        &mut self,
        prefix: QualifiedName<'a>,
    ) -> ParseResult<'a, Type<'a>> {
        let ctx = ("annotation_decl", self.peek_next_token().addr);

        // verify @interface
        consume_token!(self, ctx, At, "@");
        consume_token!(self, ctx, Keyword("interface"), "interface");

        let name = if let Identifier(s) = self.get_next_token().token {
            let mut v = prefix.clone();
            v.0.push(s);
            v
        } else {
            return Err(ParseErrType::UnexpectedToken {
                expected: "IDENTIFIER",
                got: vec![self.get_current_token().token],
            }
            .to_stack_parse_err(self.get_current_token().addr, ctx));
        };

        let (type_kind, body) = self.annotation_body(name.clone()).push_context(ctx)?;

        Ok(Type {
            name,
            type_kind,
            body,
            annotation: vec![],
            modifiers: Modifiers {
                modifiers: vec![],
                access_modifier: AccessModifier::Default,
            },
        })
    }

    pub(crate) fn annotation_body(
        &mut self,
        prefix: QualifiedName<'a>,
    ) -> ParseResult<'a, (TypeKind<'a>, TypeBody<'a>)> {
        let ctx = ("annotation_body", self.peek_next_token().addr);
        let mut annotation_elements: Vec<(&'a str, RefType<'a>)> = vec![];
        let mut body: TypeBody = TypeBody {
            members: vec![],
            subtypes: vec![],
        };

        consume_token!(self, ctx, LBrace, "LBrace");

        loop {
            match self.peek_next_token().token {
                RBrace => {
                    self.get_next_token();
                    break;
                }
                At | Keyword(_) | Identifier(_) => {
                    let annotations = self.annotations().push_context(ctx)?;
                    self.modifiers().push_context(ctx)?; // dont care for this, just for safety.
                    let modifiers = Modifiers {
                        modifiers: vec!["static", "final"],
                        access_modifier: AccessModifier::Public,
                    };

                    // IDENTIFIER -> RefType
                    // @interface | class | enum | interface -> subtype
                    match (
                        self.peek_next_token().token,
                        self.peek_token_offset(1).token,
                    ) {
                        (Keyword("enum"), _) => {
                            let mut subtype = self.enum_decl(prefix.clone()).push_context(ctx)?;
                            subtype.annotation = annotations;
                            subtype.modifiers = modifiers;
                            body.subtypes.push(subtype);
                        }
                        (Keyword("class"), _) => {
                            let mut subtype = self.class_decl(prefix.clone()).push_context(ctx)?;
                            subtype.annotation = annotations;
                            subtype.modifiers = modifiers;
                            body.subtypes.push(subtype);
                        }
                        (Keyword("interface"), _) => {
                            let mut subtype =
                                self.interface_decl(prefix.clone()).push_context(ctx)?;
                            subtype.annotation = annotations;
                            subtype.modifiers = modifiers;
                            body.subtypes.push(subtype);
                        }
                        (At, Keyword("interface")) => {
                            let mut subtype =
                                self.annotation_decl(prefix.clone()).push_context(ctx)?;
                            subtype.annotation = annotations;
                            subtype.modifiers = modifiers;
                            body.subtypes.push(subtype);
                        }
                        (Identifier(_), _) => {
                            let typeclass = self.ref_type().push_context(ctx)?;
                            let name = if let Identifier(s) = self.get_next_token().token {
                                let mut qualified_name = prefix.clone();
                                qualified_name.0.push(s);
                                qualified_name
                            } else {
                                return Err(ParseErrType::UnexpectedToken {
                                    expected: "IDENTIFIER",
                                    got: vec![self.get_current_token().token],
                                }
                                .to_stack_parse_err(self.get_current_token().addr, ctx));
                            };
                            match self.get_next_token().token {
                                Assignment("=") => {
                                    // we skip until we see ";"
                                    // don't actually need to consume "default"
                                    // since the loop naturally consumes it.
                                    loop {
                                        match self.peek_next_token().token {
                                            LParen => {
                                                self.skip_brace(LParen, RParen)
                                                    .push_context(ctx)?;
                                            }
                                            LBrace => {
                                                self.skip_brace(LBrace, RBrace)
                                                    .push_context(ctx)?;
                                            }
                                            LBracket => {
                                                self.skip_brace(LBracket, RBracket)
                                                    .push_context(ctx)?;
                                            }
                                            Semicolon => break,
                                            EOF => {
                                                return Err(ParseErrType::UnexpectedEOF
                                                    .to_stack_parse_err(
                                                        self.peek_next_token().addr,
                                                        ctx,
                                                    ));
                                            }
                                            _ => {
                                                self.get_next_token();
                                            }
                                        }
                                    }
                                    consume_token!(self, ctx, Semicolon, "Semicolon");
                                    body.members.push(Member {
                                        name: name.0.last().copied().unwrap(),
                                        member_kind: MemberKind::Property { reftype: typeclass },
                                        annotations,
                                        modifiers,
                                    });
                                }
                                LParen => {
                                    consume_token!(self, ctx, RParen, "RParen");

                                    if self.peek_next_token().token == Keyword("default") {
                                        // we skip until we see ";"
                                        // don't actually need to consume "default"
                                        // since the loop naturally consumes it.
                                        loop {
                                            match self.peek_next_token().token {
                                                LParen => {
                                                    self.skip_brace(LParen, RParen)
                                                        .push_context(ctx)?;
                                                }
                                                LBrace => {
                                                    self.skip_brace(LBrace, RBrace)
                                                        .push_context(ctx)?;
                                                }
                                                LBracket => {
                                                    self.skip_brace(LBracket, RBracket)
                                                        .push_context(ctx)?;
                                                }
                                                Semicolon => break,
                                                EOF => {
                                                    return Err(ParseErrType::UnexpectedEOF
                                                        .to_stack_parse_err(
                                                            self.peek_next_token().addr,
                                                            ctx,
                                                        ));
                                                }
                                                _ => {
                                                    self.get_next_token();
                                                }
                                            }
                                        }
                                    }

                                    consume_token!(self, ctx, Semicolon, "Semicolon");
                                    annotation_elements
                                        .push((name.0.last().copied().unwrap(), typeclass));
                                }
                                token => {
                                    return Err(ParseErrType::UnexpectedToken {
                                        expected: "LParen | =",
                                        got: vec![token],
                                    }
                                    .to_stack_parse_err(self.get_current_token().addr, ctx));
                                }
                            }
                        }
                        (token1, token2) => {
                            return Err(ParseErrType::UnexpectedToken {
                                expected: "type_decl | IDENTIFIER",
                                got: vec![token1, token2],
                            }
                            .to_stack_parse_err(self.peek_next_token().addr, ctx));
                        }
                    }
                }

                token => {
                    // <ref_type> IDENTIFIER (
                    //  | "class" | "interface" | "enum" | "@interface"
                    //  | "()" ["default" <skip> ";"]
                    //  | "=" <skip> ";"
                    // )
                    return Err(ParseErrType::UnexpectedToken {
                        expected: "@ | IDENTIFIER | RBrace",
                        got: vec![token],
                    }
                    .to_stack_parse_err(self.peek_next_token().addr, ctx));
                }
            }
        }

        Ok((
            TypeKind::Annotation {
                annotation_properties: annotation_elements,
            },
            body,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_annotation() {
        let mut parser = Parser::new(
            "@Retention(RetentionPolicy.RUNTIME)
           @Target({ElementType.METHOD, ElementType.TYPE})
           public @interface MyAnnotation {
               String value();
               int count() default 1;
               String[] tags() default {};
               int MAX = 100;
               enum Scope { CLASS, METHOD }
               Scope scope() default Scope.CLASS;
           }",
        )
        .unwrap();
        let res: Type = parser.type_decl(QualifiedName(vec![])).unwrap();
        // println!("res:\n {:#?}", res);

        assert_eq!(res.name, QualifiedName(vec!["MyAnnotation"]));
        assert_eq!(res.modifiers.access_modifier, AccessModifier::Public);

        let TypeKind::Annotation {
            annotation_properties,
        } = &res.type_kind
        else {
            panic!("expected Annotation");
        };

        // elements: (name, type)
        assert_eq!(annotation_properties[0].0, "value");
        assert_eq!(annotation_properties[1].0, "count");
        assert_eq!(annotation_properties[2].0, "tags");
        assert_eq!(annotation_properties[3].0, "scope");

        // constant
        assert_eq!(res.body.members[0].name, "MAX");
        assert!(matches!(
            res.body.members[0].member_kind,
            MemberKind::Property { .. }
        ));

        // nested enum
        assert_eq!(
            res.body.subtypes[0].name,
            QualifiedName(vec!["MyAnnotation", "Scope"])
        );
        assert!(matches!(
            res.body.subtypes[0].type_kind,
            TypeKind::Enum { .. }
        ));
    }
}
