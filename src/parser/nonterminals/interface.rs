use std::collections::BTreeSet;

use super::super::{parser::Parser, token::Token::*};
use crate::types::*;

// ---------------------------------------------------------------------
// ----------------------- Interface Nonterminals ----------------------
// ---------------------------------------------------------------------
impl<'a> Parser<'a> {
    /// ``` ebnf
    /// <interface_decl> ::= "interface" IDENTIFIER
    ///         <type_param_list>
    ///         ["extends" <ref_type> {"," <ref_type>}]
    ///         ["permits" <ref_type> {"," <ref_type>}]
    ///         <interface_body>
    /// ```
    pub(crate) fn interface_decl(&mut self, prefix: QualifiedName) -> ParseResult<Type> {
        let ctx = ("interface_decl", self.peek_next_token().addr);

        // "interface" IDENFITIER <type_param_list>
        if self.get_next_token().token != Keyword("interface") {
            return Err(ParseErrType::UnexpectedToken {
                expected: "interface",
                got: vec![self.get_current_token().token.to_owned_token()],
            }
            .to_stack_parse_err(self.get_current_token().addr, ctx));
        }
        let name = if let Identifier(s) = self.get_next_token().token {
            let mut v = prefix.clone();
            v.0.push(s.to_string());
            v
        } else {
            return Err(ParseErrType::UnexpectedToken {
                expected: "IDENTIFIER",
                got: vec![self.get_current_token().token.to_owned_token()],
            }
            .to_stack_parse_err(self.get_current_token().addr, ctx));
        };
        self.type_param_list().push_context(ctx)?;

        // ["extends" <ref_type> {"," <ref_type>}]
        let extend_interfaces = if self.peek_next_token().token == Keyword("extends") {
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

        // ["permits" <ref_type> {"," <ref_type>}]
        if self.peek_next_token().token == Keyword("permits") {
            self.get_next_token();
            self.ref_type().push_context(ctx)?;
            while self.peek_next_token().token == Comma {
                self.get_next_token();
                self.ref_type().push_context(ctx)?;
            }
        }

        let body = self
            .interface_body(name.clone(), name.0.last().unwrap().to_owned())
            .push_context(ctx)?;

        Ok(Type {
            name,
            body,
            type_kind: TypeKind::Interface { extend_interfaces },
            annotation: vec![],
            modifiers: Modifiers {
                modifiers: BTreeSet::new(),
                access_modifier: AccessModifier::Default,
            },
        })
    }

    /// Essentially members, maybe with member filters/checkers
    pub(crate) fn interface_body(
        &mut self,
        prefix: QualifiedName,
        classname: String,
    ) -> ParseResult<TypeBody> {
        let ctx = ("interface_body", self.peek_next_token().addr);

        // typical "{" <members> "}"
        consume_token!(self, ctx, LBrace, "LBrace");
        let mut body = self.members(prefix, classname).push_context(ctx)?;
        consume_token!(self, ctx, RBrace, "RBrace");

        // Verify these stuffs:
        // - No constructor
        for member in body.members.iter() {
            if let MemberKind::Constructor { .. } = member.member_kind.clone() {
                return Err(ParseErrType::SemanticError("Constructor in interface")
                    .to_stack_parse_err(ctx.1, ctx));
            }
        }

        // Add public static to inner types
        for inner in body.subtypes.iter_mut() {
            inner.modifiers.modifiers.insert("static".to_owned());
            inner.modifiers.access_modifier =
                if inner.modifiers.access_modifier == AccessModifier::Default {
                    AccessModifier::Public
                } else {
                    inner.modifiers.access_modifier
                };
        }

        while self.peek_next_token().token == Semicolon {
            self.get_next_token();
        }
        Ok(body)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_interface() {
        let mut parser = Parser::new(
            "
            // Can be used for different types of vector or representations
            // maybe a (r, theta) or (x, y) or spherical plane geometry.
            public interface MetricSpaceTwoValueVector {
                public static class Point {
                    float p1 = 0, p2 = 0;
                    public void setPoint(float p1, float p2) {
                        this.p1 = p1;
                        this.p2 = p2;
                    }
                    public float[] getPoint() {
                        return {p1, p2};
                    }
                }
                
                public float getDistance(Point x, Point y);
                public float getNorm(Point x);
            }
        ",
        )
        .unwrap();
        let res = parser.type_decl(QualifiedName(vec![])).unwrap();
        // println!("res:\n {:#?}", res);
    }
}
