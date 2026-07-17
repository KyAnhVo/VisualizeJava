use super::super::{parser::Parser, token::Token::*};
use crate::types::*;
use std::rc::Rc;

impl<'a> Parser<'a> {
    /// `<type_body>      ::= "{" {<member_decl>} "}"`, where
    /// `<member_decl>     ::= {<annotation>} <modifiers> ( <method_decl> | <constructor_decl> | <property_decl> | <type_decl> )`
    pub(crate) fn members(
        &mut self,
        prefix: QualifiedName,
        classname: String,
    ) -> ParseResult<TypeBody> {
        let ctx = ("members", self.peek_next_token().addr);

        // if the next token is not closing the body, then it must be still
        // a member.
        let mut body = TypeBody {
            members: vec![],
            subtypes: vec![],
        };

        // {<member_decl>}, inside is the <member_decl>
        while self.peek_next_token().token != RBrace {
            if self.peek_next_token().token == EOF {
                return Err(ParseErrType::UnexpectedEOF
                    .to_stack_parse_err(self.get_current_token().addr, ctx));
            }

            if self.peek_next_token().token == Semicolon {
                self.get_next_token();
                continue;
            }

            let annotations = self.annotations().push_context(ctx)?;
            let modifiers = self.modifiers().push_context(ctx)?;

            match (
                self.peek_next_token().token,
                self.peek_token_offset(1).token,
            ) {
                // initializer block
                (LBrace, _) => {
                    self.skip_brace(LBrace, RBrace).push_context(ctx)?;
                }
                // Types: class
                (Keyword("class"), _) => {
                    let mut typeclass = self.class_decl(prefix.clone()).push_context(ctx)?;
                    typeclass.modifiers = modifiers;
                    typeclass.annotation = annotations;
                    body.subtypes.push(Rc::new(typeclass));
                }
                // Types: enum
                (Keyword("enum"), _) => {
                    let mut typeclass = self.enum_decl(prefix.clone()).push_context(ctx)?;
                    typeclass.modifiers = modifiers;
                    typeclass.annotation = annotations;
                    body.subtypes.push(Rc::new(typeclass));
                }
                // Types: annotation
                (At, Keyword("interface")) => {
                    let mut typeclass = self.annotation_decl(prefix.clone()).push_context(ctx)?;
                    typeclass.modifiers = modifiers;
                    typeclass.annotation = annotations;
                    body.subtypes.push(Rc::new(typeclass));
                }

                // Types: interface
                (Keyword("interface"), _) => {
                    let mut typeclass = self.interface_decl(prefix.clone()).push_context(ctx)?;
                    typeclass.modifiers = modifiers;
                    typeclass.annotation = annotations;
                    body.subtypes.push(Rc::new(typeclass));
                }
                // Members: method with type_param
                (LessThan, _) => {
                    // <type_param_list> <voidable_type> IDENTIFIER <type_arg_list> <arg_list> <method_body>
                    let type_param_list = self.type_param_list().push_context(ctx)?;
                    if self.peek_next_token().token == Identifier(classname.as_str())
                        && self.peek_token_offset(1).token == LParen
                    {
                        let name = match self.get_next_token().token {
                            Identifier(s) => s,
                            _ => unreachable!(),
                        };
                        let input = self.arg_list().push_context(ctx)?;
                        let throws = if self.peek_next_token().token == Keyword("throws") {
                            let mut v: Vec<RefType> = vec![];
                            self.get_next_token();
                            v.push(self.ref_type().push_context(ctx)?);
                            while self.peek_next_token().token == Comma {
                                self.get_next_token();
                                v.push(self.ref_type().push_context(ctx)?);
                            }
                            v
                        } else {
                            vec![]
                        };
                        // must have body, since this is a constructor
                        self.skip_brace(LBrace, RBrace).push_context(ctx)?;
                        body.members.push(Rc::new(Member {
                            name: name.to_owned(),
                            member_kind: MemberKind::Constructor {
                                type_param_list,
                                input,
                                throws,
                            },
                            annotations,
                            modifiers,
                        }))
                    } else {
                        let output = self.voidable_type().push_context(ctx)?;
                        let name = if let Identifier(s) = self.get_next_token().token {
                            s
                        } else {
                            return Err(ParseErrType::UnexpectedToken {
                                expected: "IDENTIFIER",
                                got: vec![self.get_current_token().token.to_owned_token()],
                            }
                            .to_stack_parse_err(self.get_current_token().addr, ctx));
                        };
                        let input = self.arg_list().push_context(ctx)?;
                        let throws = if self.peek_next_token().token == Keyword("throws") {
                            let mut v: Vec<RefType> = vec![];

                            // "throws" <ref_type>
                            self.get_next_token();
                            v.push(self.ref_type().push_context(ctx)?);

                            // {"," <ref_type>}
                            while self.peek_next_token().token == Comma {
                                self.get_next_token();
                                v.push(self.ref_type().push_context(ctx)?);
                            }

                            v
                        } else {
                            vec![]
                        };
                        match self.peek_next_token().token {
                            Semicolon => {
                                self.get_next_token();
                            }
                            LBrace => self.skip_brace(LBrace, RBrace).push_context(ctx)?,
                            token => {
                                return Err(ParseErrType::UnexpectedToken {
                                    expected: "Semicolon | LBrace",
                                    got: vec![token.to_owned_token()],
                                }
                                .to_stack_parse_err(self.get_current_token().addr, ctx));
                            }
                        }
                        body.members.push(Rc::new(Member {
                            name: name.to_owned(),
                            member_kind: MemberKind::Method {
                                type_param_list,
                                input,
                                output,
                                throws,
                            },
                            annotations,
                            modifiers,
                        }))
                    }
                }
                // `classname <arg_list> ["throws" <ref_type> {"," <ref_type>}] <method_body>`
                (Identifier(s), LParen) if s == classname => {
                    let name = match self.get_next_token().token {
                        Identifier(s) => s,
                        _ => unreachable!(),
                    };
                    let input = self.arg_list().push_context(ctx)?;
                    let throws = if self.peek_next_token().token == Keyword("throws") {
                        self.get_next_token();
                        let mut v: Vec<RefType> = vec![];
                        v.push(self.ref_type().push_context(ctx)?);
                        while self.peek_next_token().token == Comma {
                            self.get_next_token();
                            v.push(self.ref_type().push_context(ctx)?);
                        }
                        v
                    } else {
                        vec![]
                    };
                    // must have body
                    self.skip_brace(LBrace, RBrace).push_context(ctx)?;

                    // donzo
                    body.members.push(Rc::new(Member {
                        name: name.to_owned(),
                        member_kind: MemberKind::Constructor {
                            type_param_list: TypeParamList(vec![]),
                            input,
                            throws,
                        },
                        annotations,
                        modifiers,
                    }))
                }
                // Members: either property or method
                (Keyword("void"), _) | (Identifier(_), _) | (At, _) => {
                    // <ref_type> <annotations> IDENTIFIER (
                    //  | <arg_list> <method_body>
                    //  | [<assignment>] {"," IDENTIFIER [<assignment>]} ";"
                    // )
                    let output = self.voidable_type().push_context(ctx)?;
                    let reftype = if let VoidableType::RefType(s) = output.clone() {
                        Ok(s)
                    } else {
                        Err(ParseErrType::UnexpectedToken {
                            expected: "IDENTIFIER",
                            got: vec![Keyword("void").to_owned_token()],
                        }
                        .to_stack_parse_err(self.get_current_token().addr, ctx))
                    };
                    let name = if let Identifier(s) = self.get_next_token().token {
                        s
                    } else {
                        return Err(ParseErrType::UnexpectedToken {
                            expected: "IDENTIFIER",
                            got: vec![self.get_current_token().token.to_owned_token()],
                        }
                        .to_stack_parse_err(self.get_current_token().addr, ctx));
                    };
                    match self.peek_next_token().token {
                        LParen => {
                            let input = self.arg_list().push_context(ctx)?;
                            let throws = if self.peek_next_token().token == Keyword("throws") {
                                let mut v: Vec<RefType> = vec![];

                                // "throws" <ref_type>
                                self.get_next_token();
                                v.push(self.ref_type().push_context(ctx)?);

                                // {"," <ref_type>}
                                while self.peek_next_token().token == Comma {
                                    self.get_next_token();
                                    v.push(self.ref_type().push_context(ctx)?);
                                }

                                v
                            } else {
                                vec![]
                            };
                            if self.peek_next_token().token == Semicolon {
                                self.get_next_token();
                            } else if self.peek_next_token().token == LBrace {
                                self.skip_brace(LBrace, RBrace).push_context(ctx)?;
                            } else {
                                return Err(ParseErrType::UnexpectedToken {
                                    expected: "Semicolon | LBrace",
                                    got: vec![self.peek_next_token().token.to_owned_token()],
                                }
                                .to_stack_parse_err(self.peek_next_token().addr, ctx));
                            }
                            body.members.push(
                                Member {
                                    name: name.to_owned(),
                                    member_kind: MemberKind::Method {
                                        type_param_list: TypeParamList(vec![]),
                                        input,
                                        output,
                                        throws,
                                    },
                                    annotations,
                                    modifiers,
                                }
                                .into(),
                            );
                        }
                        Assignment("=") | Comma | Semicolon | LBracket => {
                            let arr_dim = {
                                let mut x = reftype.clone()?.arr_dim;
                                while self.peek_next_token().token == LBracket {
                                    consume_token!(self, ctx, LBracket, "LBracket");
                                    consume_token!(self, ctx, RBracket, "RBracket");
                                    x += 1;
                                }
                                x
                            };

                            // resolve assignment
                            // = <skip_assignment> {"," IDENTIFIER ["=" <skip_assignment>]} ";"

                            // "=" <skip_assignment>
                            if self.peek_next_token().token == Assignment("=") {
                                loop {
                                    match self.peek_next_token().token {
                                        LBrace => {
                                            self.skip_brace(LBrace, RBrace).push_context(ctx)?
                                        }
                                        LParen => {
                                            self.skip_brace(LParen, RParen).push_context(ctx)?
                                        }
                                        LBracket => {
                                            self.skip_brace(LBracket, RBracket).push_context(ctx)?
                                        }
                                        Semicolon => {
                                            break;
                                        }
                                        Comma
                                            if self
                                                .check_end_assignment_comma()
                                                .push_context(ctx)? =>
                                        {
                                            break;
                                        }
                                        _ => {
                                            self.get_next_token();
                                        }
                                    };
                                }
                            }

                            body.members.push(
                                Member {
                                    name: name.to_owned(),
                                    member_kind: MemberKind::Property {
                                        reftype: reftype.clone()?,
                                        arr_dim,
                                    },
                                    annotations: annotations.clone(),
                                    modifiers: modifiers.clone(),
                                }
                                .into(),
                            );

                            // {"," IDENTIFIER ["=" <skip_assignment>]} ";"
                            while self.peek_next_token().token == Comma {
                                self.get_next_token();
                                // IDENTIFIER
                                let Identifier(name) = self.get_next_token().token else {
                                    return Err(ParseErrType::UnexpectedToken {
                                        expected: "IDENTIFIER",
                                        got: vec![self.get_current_token().token.to_owned_token()],
                                    }
                                    .to_stack_parse_err(self.get_current_token().addr, ctx));
                                };

                                let arr_dim = {
                                    let mut x = reftype.clone()?.arr_dim;
                                    while self.peek_next_token().token == LBracket {
                                        self.get_next_token();
                                        consume_token!(self, ctx, RBracket, "RBracket");
                                        x += 1;
                                    }
                                    x
                                };

                                match self.peek_next_token().token {
                                    Assignment("=") => loop {
                                        match self.peek_next_token().token {
                                            LBrace => {
                                                self.skip_brace(LBrace, RBrace).push_context(ctx)?
                                            }
                                            LParen => {
                                                self.skip_brace(LParen, RParen).push_context(ctx)?
                                            }
                                            LBracket => self
                                                .skip_brace(LBracket, RBracket)
                                                .push_context(ctx)?,
                                            Semicolon => break,
                                            Comma
                                                if self
                                                    .check_end_assignment_comma()
                                                    .push_context(ctx)? =>
                                            {
                                                break;
                                            }
                                            _ => {
                                                self.get_next_token();
                                            }
                                        };
                                    },
                                    Semicolon | Comma => {}
                                    token => {
                                        return Err(ParseErrType::UnexpectedToken {
                                            expected: "Assignment | Semicolon | Comma",
                                            got: vec![token.to_owned_token()],
                                        }
                                        .to_stack_parse_err(self.peek_next_token().addr, ctx));
                                    }
                                };

                                body.members.push(
                                    Member {
                                        name: name.to_owned(),
                                        member_kind: MemberKind::Property {
                                            reftype: reftype.clone()?,
                                            arr_dim,
                                        },
                                        annotations: annotations.clone(),
                                        modifiers: modifiers.clone(),
                                    }
                                    .into(),
                                );
                            }
                            consume_token!(self, ctx, Semicolon, "Semicolon");
                        }
                        token => {
                            return Err(ParseErrType::UnexpectedToken {
                                expected: "LBrace | = | Comma",
                                got: vec![token.to_owned_token()],
                            }
                            .to_stack_parse_err(self.peek_next_token().addr, ctx));
                        }
                    }
                }
                // error
                (token1, _) => {
                    return Err(ParseErrType::UnexpectedToken {
                        expected: "type_decl | type_param",
                        got: vec![token1.to_owned_token()],
                    }
                    .to_stack_parse_err(self.peek_next_token().addr, ctx));
                }
            };
        }
        // consume the RBrace
        Ok(body)
    }
}
