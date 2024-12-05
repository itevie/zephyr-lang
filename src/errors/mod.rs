use std::fs;

use crate::{lexer::tokens::Location, runtime::values::RuntimeValue, util};

#[derive(Debug, Clone)]
pub enum ErrorCode {
    LexerError,
    UnexpectedCharacter,
    UnterminatedString,
    CannotEscape,

    UnexpectedToken,
    InvalidNumber,

    RuntimeError,
    UnknownReference,
    InvalidOperation,
    CannotCoerce,
    AlreadyDefined,
    ConstantAssignment,
    InternalLockError,
    ScopeError,
    OutOfBounds,
    InvalidKey,

    Break,
    Continue,
    Return(Option<RuntimeValue>),
}

#[derive(Debug, Clone)]
pub struct ZephyrError {
    pub code: ErrorCode,
    pub message: String,
    pub location: Option<Location>,
}

impl ZephyrError {
    pub fn visualise(&self) -> String {
        let mut string = format!(
            "{}{:?} error: {}{}",
            util::FG_RED,
            self.code,
            self.message,
            util::COLOR_RESET
        );

        if let Some(ref location) = self.location {
            let mut file_contents: Result<String, String> = Err("<no file provided>".to_string());
            if let Some(ref file_name) = location.file_name {
                file_contents = match fs::read_to_string(file_name) {
                    Ok(ok) => Ok(ok),
                    Err(err) => Err(format!("<failed to read {}: {}>", file_name, err.kind())),
                }
            }

            let result = if let Ok(contents) = file_contents {
                let lines = contents.split('\n').collect::<Vec<&str>>();
                if location.line >= lines.len() {
                    "<invalid location>".to_string()
                } else {
                    lines[location.line].to_string()
                }
            } else {
                file_contents.unwrap_err()
            };

            string.push_str(&format!(
                "\n\n{}{}{}",
                util::FG_GRAY,
                result,
                util::COLOR_RESET
            ));

            string.push_str(&format!(
                "\n{}{}{}{}",
                util::FG_CYAN,
                " ".repeat(location.start),
                "^".repeat(location.end - location.start),
                util::COLOR_RESET
            ));
        }

        return string;
    }
}
