use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct Token {
    pub value: String,
    pub t: TokenType,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub enum TokenType {
    OpenParan,
    CloseParan,
    OpenSquare,
    CloseSquare,
    OpenBrace,
    CloseBrace,
    Comma,
    Dot,
    Colon,
    Semicolon,
    QuestionMark,
    Arrow,

    EOF,

    Number,
    String,
    Symbol,

    Assign,

    Additive(Additive),
    Multiplicative(Multiplicative),
    Logical(Logical),
    Unary(Unary),
    Comparison(Comparison),

    Let,
    Const,

    Import,
    Export,
    As,

    Try,
    Catch,
    Finally,
    Throw,

    For,
    While,
    Break,
    Continue,

    Function,
    Return,
    Where,

    If,
    Else,
    Match,
}

#[derive(Debug, Clone)]
pub enum Additive {
    Plus,
    Minus,
}

#[derive(Debug, Clone)]
pub enum Multiplicative {
    Multiply,
    Divide,
    Modulo,
}

#[derive(Debug, Clone)]
pub enum Logical {
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum Unary {
    Not,
    Length,
    Increment,
    Decrement,
}

#[derive(Debug, Clone)]
pub enum Comparison {
    Eq,
    Neq,
    Gt,
    Lt,
    GtEq,
    LtEq,
}

impl Display for Comparison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Comparison::Eq => "==",
                Comparison::Neq => "!=",
                Comparison::Gt => ">",
                Comparison::Lt => "<",
                Comparison::GtEq => ">=",
                Comparison::LtEq => "<=",
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct Location {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub file_name: Option<String>,
}

pub static NO_LOCATION: Location = Location {
    start: 0,
    end: 0,
    line: 0,
    file_name: None,
};
