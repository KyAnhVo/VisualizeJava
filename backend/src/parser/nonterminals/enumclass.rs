use std::{collections::BTreeSet, rc::Rc};

use super::super::{parser::Parser, token::Token::*};
use crate::types::*;

// ---------------------------------------------------------------------
// ----------------------- Enum Nonterminals ---------------------------
// ---------------------------------------------------------------------

impl<'a> Parser<'a> {
    pub(crate) fn enum_decl(&mut self, prefix: QualifiedName) -> ParseResult<Type> {
        let ctx = ("enum_decl", self.peek_next_token().addr);

        if self.get_next_token().token != Keyword("enum") {
            return Err(ParseErrType::UnexpectedToken {
                expected: "enum",
                got: vec![self.get_current_token().token.to_owned_token()],
            }
            .to_stack_parse_err(self.get_current_token().addr, ctx));
        }

        let name = if let Identifier(s) = self.get_next_token().token {
            let mut v = prefix.clone();
            v.0.push(s.to_owned());
            v
        } else {
            return Err(ParseErrType::UnexpectedToken {
                expected: "IDENTIFIER",
                got: vec![self.get_current_token().token.to_owned_token()],
            }
            .to_stack_parse_err(self.get_current_token().addr, ctx));
        };

        let implement_interfaces: Vec<RefType> =
            if self.peek_next_token().token == Keyword("implements") {
                let mut v = vec![];
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

        let (enum_vals, mut body) = self
            .enum_body(name.clone(), name.0.last().unwrap().to_owned())
            .push_context(ctx)?;

        body.subtypes = body
            .subtypes
            .iter()
            .map(|subtype| {
                let mut modifiers = subtype.modifiers.clone();
                modifiers.modifiers.insert("static".to_owned());
                Rc::new(Type {
                    name: subtype.name.clone(),
                    type_params: TypeParamList(vec![]),
                    modifiers,
                    type_kind: subtype.type_kind.clone(),
                    annotation: subtype.annotation.clone(),
                    body: subtype.body.clone(),
                })
            })
            .collect();

        Ok(Type {
            name,
            type_params: TypeParamList(vec![]),
            modifiers: Modifiers {
                modifiers: BTreeSet::new(),
                access_modifier: AccessModifier::Default,
            },
            type_kind: TypeKind::Enum {
                implement_interfaces,
                enum_vals,
            },
            body,
            annotation: vec![].into(),
        })
    }

    /// ```
    /// <enum_body> ::= "{" [<enum_val> {"," <enum_val>}] [";" <members>] "}"
    /// ```
    /// where
    /// ```
    /// <enum_val> ::= IDENTIFIEIR [<skip_paren>] [<skip_brace>]
    /// ```
    pub(crate) fn enum_body(
        &mut self,
        prefix: QualifiedName,
        classname: String,
    ) -> ParseResult<(Vec<String>, TypeBody)> {
        let ctx = ("enum_body", self.peek_next_token().addr);

        consume_token!(self, ctx, LBrace, "LBrace");

        let mut enum_vals: Vec<String> = vec![];
        self.annotations()?;
        if let Identifier(s) = self.peek_next_token().token {
            self.get_next_token();
            enum_vals.push(s.to_owned());
            if self.peek_next_token().token == LParen {
                self.skip_brace(LParen, RParen).push_context(ctx)?;
            }
            if self.peek_next_token().token == LBrace {
                self.skip_brace(LBrace, RBrace).push_context(ctx)?;
            }

            while self.peek_next_token().token == Comma {
                self.get_next_token();
                self.annotations()?;
                let Identifier(s) = self.peek_next_token().token else {
                    break;
                };
                self.get_next_token();
                enum_vals.push(s.to_owned());
                if self.peek_next_token().token == LParen {
                    self.skip_brace(LParen, RParen).push_context(ctx)?;
                }
                if self.peek_next_token().token == LBrace {
                    self.skip_brace(LBrace, RBrace).push_context(ctx)?;
                }
            }
        }

        let body = match self.get_next_token().token {
            RBrace => TypeBody {
                members: vec![].into(),
                subtypes: vec![].into(),
            },
            Semicolon => {
                let res = self.members(prefix, classname).push_context(ctx)?;
                if self.get_next_token().token != RBrace {
                    return Err(ParseErrType::UnexpectedToken {
                        expected: "LBrace",
                        got: vec![self.get_current_token().token.to_owned_token()],
                    }
                    .to_stack_parse_err(self.get_current_token().addr, ctx));
                }
                res
            }
            token => {
                return Err(ParseErrType::UnexpectedToken {
                    expected: "RBrace | Semicolon",
                    got: vec![token.to_owned_token()],
                }
                .to_stack_parse_err(self.get_current_token().addr, ctx));
            }
        };

        while self.peek_next_token().token == Semicolon {
            self.get_next_token();
        }
        Ok((enum_vals, body))
    }
}
