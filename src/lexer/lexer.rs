use crate::errors::{ErrorCode, ZephyrError};

use super::tokens::{
    Additive, Comparison, Location, Logical, Multiplicative, Token, TokenType, Unary,
};

pub fn lex(contents: &str, file_name: String) -> Result<Vec<Token>, ZephyrError> {
    let mut tokens: Vec<Token> = vec![];
    let mut chars = contents.chars().peekable();

    let mut current_line: usize = 0;
    let mut current_char: usize = 0;

    while let Some(char) = chars.next() {
        let mut current_location = Location {
            start: current_char,
            end: current_char,
            line: current_line,
            file_name: Some(file_name.clone()),
        };
        let current_value: String;
        let current_token: Option<TokenType>;
        let mut current_length: usize = 0;

        match char {
            '\n' => {
                current_line += 1;
                current_char = 0;
                continue;
            }
            ' ' | '\r' | '\t' => {
                current_char += 1;
                continue;
            }

            '/' => {
                if let Some(next_char) = chars.peek() {
                    match next_char {
                        '/' => {
                            // Single-line comment
                            while let Some(c) = chars.next() {
                                if c == '\n' {
                                    break;
                                }
                            }
                            current_line += 1;
                            current_char = 0;
                            continue;
                        }
                        '*' => {
                            chars.next();
                            while let Some(c) = chars.next() {
                                if c == '*' {
                                    if let Some('/') = chars.peek() {
                                        chars.next();
                                        break;
                                    }
                                }
                                if c == '\n' {
                                    current_line += 1;
                                    current_char = 0;
                                }
                            }
                            current_char += 2;
                            continue;
                        }
                        _ => {}
                    }
                }
                current_token = Some(TokenType::Multiplicative(Multiplicative::Divide));
                current_length = 1;
                current_value = char.to_string();
            }

            _ if char.is_numeric() => {
                let mut value = char.to_string();
                while chars.peek().unwrap_or(&'n').is_numeric() {
                    value.push(chars.next().unwrap())
                }

                current_token = Some(TokenType::Number);
                current_length = value.len();
                current_value = value;
            }

            '"' => {
                let mut value = String::new();
                while let Some(char) = chars.peek() {
                    if *char == '"' || *char == '\n' {
                        break;
                    }

                    if *char == '\\' {
                        chars.next();
                        if let Some(to_escape) = chars.next() {
                            let result = match to_escape {
                                '"' => '"',
                                '\\' => '\\',
                                'n' => '\n',
                                't' => '\t',
                                'r' => '\r',
                                _ => {
                                    return Err(ZephyrError {
                                        code: ErrorCode::CannotEscape,
                                        message: format!("Cannot escape {}", to_escape),
                                        location: Some(current_location),
                                    })
                                }
                            };

                            current_length += 2;
                            value.push(result);
                            continue;
                        } else {
                            return Err(ZephyrError {
                                code: ErrorCode::CannotEscape,
                                message: String::from("Nothing to escape"),
                                location: Some(current_location),
                            });
                        }
                    }

                    current_length += 1;
                    value.push(chars.next().unwrap());
                }

                if let None = chars.next() {
                    return Err(ZephyrError {
                        code: ErrorCode::UnterminatedString,
                        message: String::from("String not closed"),
                        location: Some(current_location),
                    });
                }

                current_token = Some(TokenType::String);
                current_length += 2;
                current_value = value;
            }

            _ if char.is_alphabetic() || char == '_' => {
                let mut value = String::from(char);
                while let Some(char) = chars.peek() {
                    if !char.is_alphanumeric() && *char != '_' {
                        break;
                    }
                    value.push(chars.next().unwrap());
                }

                if *chars.peek().unwrap_or(&'n') == '!' {
                    value.push(chars.next().unwrap());
                }

                current_token = Some(match value.as_str() {
                    "let" => TokenType::Let,
                    "const" => TokenType::Const,

                    "try" => TokenType::Try,
                    "catch" => TokenType::Catch,
                    "finally" => TokenType::Finally,
                    "throw" => TokenType::Throw,

                    "is" => TokenType::Is,

                    "import" => TokenType::Import,
                    "export" => TokenType::Export,
                    "as" => TokenType::As,
                    "expose" => TokenType::Expose,

                    "for" => TokenType::For,
                    "in" => TokenType::In,
                    "while" => TokenType::While,
                    "continue" => TokenType::Continue,
                    "break" => TokenType::Break,

                    "func" => TokenType::Function,
                    "return" => TokenType::Return,
                    "where" => TokenType::Where,

                    "if" => TokenType::If,
                    "else" => TokenType::Else,
                    "match" => TokenType::Match,

                    "debug" => TokenType::Debug,

                    "enum" => TokenType::Enum,
                    _ => TokenType::Symbol,
                });
                current_length = value.len();
                current_value = value;
            }

            _ => {
                let mut actual_value: Option<String> = None;
                let next_char = chars.peek().copied().unwrap_or('n');

                let token = Some(match char {
                    '.' => {
                        if next_char == '.' {
                            chars.next();
                            if chars.peek().copied().unwrap_or('n') == '=' {
                                chars.next();
                                actual_value = Some(String::from("..="));
                                TokenType::RangeInclusive
                            } else {
                                actual_value = Some(String::from(".."));
                                TokenType::Range
                            }
                        } else {
                            TokenType::Dot
                        }
                    }
                    ',' => TokenType::Comma,
                    ':' => TokenType::Colon,
                    ';' => TokenType::Semicolon,
                    '(' => TokenType::OpenParan,
                    ')' => TokenType::CloseParan,
                    '[' => TokenType::OpenSquare,
                    ']' => TokenType::CloseSquare,
                    '{' => TokenType::OpenBrace,
                    '}' => TokenType::CloseBrace,
                    '?' => TokenType::QuestionMark,

                    '+' => {
                        if next_char == '+' {
                            chars.next();
                            actual_value = Some(String::from("++"));
                            TokenType::Unary(Unary::Increment)
                        } else {
                            TokenType::Additive(Additive::Plus)
                        }
                    }
                    '-' => {
                        if next_char == '-' {
                            chars.next();
                            actual_value = Some(String::from("--"));
                            TokenType::Unary(Unary::Decrement)
                        } else if next_char == '>' {
                            chars.next();
                            actual_value = Some(String::from("->"));
                            TokenType::Arrow
                        } else {
                            TokenType::Additive(Additive::Minus)
                        }
                    }

                    '*' => {
                        if next_char == '*' {
                            chars.next();
                            actual_value = Some(String::from("**"));
                            TokenType::Multiplicative(Multiplicative::Exponent)
                        } else {
                            TokenType::Multiplicative(Multiplicative::Multiply)
                        }
                    }
                    '/' => TokenType::Multiplicative(Multiplicative::Divide),
                    '%' => TokenType::Multiplicative(Multiplicative::Modulo),

                    '!' => {
                        if next_char == '=' {
                            chars.next();
                            actual_value = Some(String::from("!="));
                            TokenType::Comparison(Comparison::Neq)
                        } else {
                            TokenType::Unary(Unary::Not)
                        }
                    }
                    '$' => TokenType::Unary(Unary::Length),

                    '&' if next_char == '&' => {
                        chars.next();
                        actual_value = Some(String::from("&&"));
                        TokenType::Logical(Logical::And)
                    }
                    '|' if next_char == '|' => {
                        chars.next();
                        actual_value = Some(String::from("||"));
                        TokenType::Logical(Logical::Or)
                    }

                    '=' => {
                        if next_char == '=' {
                            chars.next();
                            actual_value = Some(String::from("=="));
                            TokenType::Comparison(Comparison::Eq)
                        } else {
                            TokenType::Assign
                        }
                    }
                    '<' => {
                        if next_char == '=' {
                            chars.next();
                            actual_value = Some(String::from("<="));
                            TokenType::Comparison(Comparison::LtEq)
                        } else {
                            TokenType::Comparison(Comparison::Lt)
                        }
                    }
                    '>' => {
                        if next_char == '=' {
                            chars.next();
                            actual_value = Some(String::from(">="));
                            TokenType::Comparison(Comparison::GtEq)
                        } else {
                            TokenType::Comparison(Comparison::Gt)
                        }
                    }
                    _ => {
                        return Err(ZephyrError {
                            code: ErrorCode::UnexpectedCharacter,
                            message: format!("Unexpected '{}'", char),
                            location: Some(current_location),
                        })
                    }
                });

                current_token = token;
                current_length = if let Some(ref v) = actual_value {
                    v.len()
                } else {
                    1
                };
                current_value = actual_value.unwrap_or_else(|| char.to_string());
            }
        }

        current_char += current_length;
        current_location.end += current_length;
        tokens.push(Token {
            t: current_token.ok_or_else(|| ZephyrError {
                code: ErrorCode::LexerError,
                message: String::from("current_token was None"),
                location: Some(current_location.clone()),
            })?,
            value: current_value,
            location: current_location,
        });
    }

    tokens.push(Token {
        t: TokenType::EOF,
        value: String::new(),
        location: Location {
            start: current_char,
            end: current_char,
            line: current_line,
            file_name: Some(file_name),
        },
    });

    Ok(tokens)
}
