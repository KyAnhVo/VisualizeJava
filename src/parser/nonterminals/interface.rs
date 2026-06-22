use super::super::{parser::Parser, token::Token::*, types::*};

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
    pub(crate) fn interface_decl(
        &mut self,
        prefix: QualifiedName<'a>,
    ) -> ParseResult<'a, Type<'a>> {
        let ctx = ("interface_decl", self.peek_next_token().addr);

        // "interface" IDENFITIER <type_param_list>
        if self.get_next_token().token != Keyword("interface") {
            return Err(ParseErrType::UnexpectedToken {
                expected: "interface",
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
        self.type_param_list().push_context(ctx)?;

        // ["extends" <ref_type> {"," <ref_type>}]
        let extend_interfaces = if self.peek_next_token().token == Keyword("extends") {
            let mut v: Vec<RefType<'a>> = vec![];
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

        Err(ParseErrType::UnimplementedError.to_stack_parse_err(ctx.1, ctx))
    }
}
