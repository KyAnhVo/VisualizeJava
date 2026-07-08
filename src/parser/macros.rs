/// Consumes the token, or return Err variant if the token is not expected
#[macro_export]
macro_rules! consume_token {
    ($self:expr, $ctx:expr, $expected:pat, $expected_str:expr) => {
        let token = $self.get_next_token();
        let $expected = token.token else {
            return Err(ParseErrType::UnexpectedToken {
                expected: $expected_str,
                got: vec![token.token.to_owned_token()],
            }
            .to_stack_parse_err(token.addr, $ctx));
        };
    };
}
