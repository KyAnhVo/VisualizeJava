use super::super::{parser::Parser, token::Token::*, types::*};

// ---------------------------------------------------------------------
// ----------------------- Annotation Nonterminals ---------------------
// ---------------------------------------------------------------------
impl<'a> Parser<'a> {
    pub(crate) fn annotation_decl(
        &mut self,
        prefix: QualifiedName<'a>,
    ) -> StackedParseResult<'a, Type<'a>> {
        let ctx = ("annotation_decl", self.peek_next_token().addr);
        Err(ParseErr::UnimplementedError.to_stack_parse_err(ctx.1, ctx))
    }
}
