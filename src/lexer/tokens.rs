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
