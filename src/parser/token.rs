#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Token<'a> {
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    LessThan,
    GreaterThan,
    Dot,
    Annotation,
    QuestionMark,
    Comma,

    /// Literals are `true`, `false`, numeric, string, char literals.
    Literal(&'a str),
    Identifier(&'a str),
    Keyword(&'a str),
    Semicolon,
    /// Assignment token includes `=`, `-=`, `+=`, `*=`, `/=`,
    /// `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=`, `>>>=`
    Assignment(&'a str),
    Op(&'a str),
    EOF,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct IndexedToken<'a> {
    pub token: Token<'a>,
    pub addr: usize,
    pub len: usize,
}
