use super::super::{parser::Parser, token::Token::*, types::*};

// ---------------------------------------------------------------------
// ----------------------- Interface Nonterminals ----------------------
// ---------------------------------------------------------------------
impl<'a> Parser<'a> {
    pub(crate) fn interface_decl(
        &mut self,
        prefix: QualifiedName<'a>,
    ) -> StackedParseResult<'a, Type<'a>> {
        let ctx = ("interface_decl", self.peek_next_token().addr);
        Err(ParseErr::UnimplementedError.to_stack_parse_err(ctx.1, ctx))
    }
}
