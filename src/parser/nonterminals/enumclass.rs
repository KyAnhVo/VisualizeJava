use super::super::{parser::Parser, token::Token::*, types::*};

// ---------------------------------------------------------------------
// ----------------------- Enum Nonterminals ---------------------------
// ---------------------------------------------------------------------

impl<'a> Parser<'a> {
    pub(crate) fn enum_decl(&mut self, prefix: QualifiedName<'a>) -> ParseResult<'a, Type<'a>> {
        let ctx = ("enum_decl", self.peek_next_token().addr);
        Err(ParseErrType::UnimplementedError.to_stack_parse_err(ctx.1, ctx))
    }
}
