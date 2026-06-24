use super::super::{parser::Parser, token::Token::*, types::*};

// ---------------------------------------------------------------------
// ----------------------- Enum Nonterminals ---------------------------
// ---------------------------------------------------------------------

impl<'a> Parser<'a> {
    pub(crate) fn enum_decl(&mut self, prefix: QualifiedName<'a>) -> ParseResult<'a, Type<'a>> {
        let ctx = ("enum_decl", self.peek_next_token().addr);

        if self.get_next_token().token != Keyword("enum") {
            return Err(ParseErrType::UnexpectedToken {
                expected: "enum",
                got: vec![self.get_current_token().token],
            }
            .to_stack_parse_err(self.get_current_token().addr, ctx));
        }

        let name = if let Identifier(s) = self.get_next_token().token {
            s
        } else {
            return Err(ParseErrType::UnexpectedToken {
                expected: "IDENTIFIER",
                got: vec![self.get_current_token().token],
            }
            .to_stack_parse_err(self.get_current_token().addr, ctx));
        };

        let implement_interfaces: Vec<RefType<'a>> =
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
        Err(ParseErrType::UnimplementedError.to_stack_parse_err(ctx.1, ctx))
    }

    pub(crate) fn enum_body(
        &mut self,
        prefix: QualifiedName<'a>,
        classname: &str,
    ) -> ParseResult<'a, TypeBody<'a>> {
        let ctx = ("enum_body", self.peek_next_token().addr);

        Err(ParseErrType::UnimplementedError.to_stack_parse_err(ctx.1, ctx))
    }
}
