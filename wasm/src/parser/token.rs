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
    At,
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

impl<'a> Token<'a> {
    pub fn to_owned_token(self) -> OwnedToken {
        match self {
            Self::LBrace => OwnedToken::LBrace,
            Self::RBrace => OwnedToken::RBrace,
            Self::LBracket => OwnedToken::LBracket,
            Self::RBracket => OwnedToken::RBracket,
            Self::LParen => OwnedToken::LParen,
            Self::RParen => OwnedToken::RParen,
            Self::LessThan => OwnedToken::LessThan,
            Self::GreaterThan => OwnedToken::GreaterThan,
            Self::Dot => OwnedToken::Dot,
            Self::At => OwnedToken::At,
            Self::QuestionMark => OwnedToken::QuestionMark,
            Self::Comma => OwnedToken::Comma,
            Self::Semicolon => OwnedToken::Semicolon,
            Self::Literal(s) => OwnedToken::Literal(s.to_owned()),
            Self::Identifier(s) => OwnedToken::Identifier(s.to_owned()),
            Self::Keyword(s) => OwnedToken::Keyword(s.to_owned()),
            Self::Assignment(s) => OwnedToken::Keyword(s.to_owned()),
            Self::Op(s) => OwnedToken::Op(s.to_owned()),
            Self::EOF => OwnedToken::EOF,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct IndexedToken<'a> {
    pub token: Token<'a>,
    pub addr: usize,
    pub len: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum OwnedToken {
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    LessThan,
    GreaterThan,
    Dot,
    At,
    QuestionMark,
    Comma,

    /// Literals are `true`, `false`, numeric, string, char literals.
    Literal(String),
    Identifier(String),
    Keyword(String),
    Semicolon,
    /// Assignment token includes `=`, `-=`, `+=`, `*=`, `/=`,
    /// `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=`, `>>>=`
    Assignment(String),
    Op(String),
    EOF,
}
