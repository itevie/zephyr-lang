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
    TypeError,
    CannotResolve,
    Unresolved,
    NotExported,
    UndefinedEventMessage,
    InvalidProperty,
    CannotIterate,
    RangeError,
    ChannelError,

    Break,
    Continue,
    Return(Option<RuntimeValue>),
    ReturnError,
}

#[derive(Debug, Clone)]
pub struct ZephyrError {
    pub code: ErrorCode,
    pub message: String,
    pub location: Option<Location>,
}

impl ZephyrError {
    pub fn _visualise(&self, file_contents: String) -> String {
        let mut string = format!(
            "{}{:?} error: {}{}",
            util::colors::FG_RED,
            if matches!(self.code, ErrorCode::Return(_)) {
                ErrorCode::ReturnError
            } else {
                self.code.clone()
            },
            self.message,
            util::colors::COLOR_RESET
        );

        if let Some(ref location) = self.location {
            let lines = file_contents.split('\n').collect::<Vec<&str>>();
            let result = if location.line >= lines.len() {
                "<invalid location>".to_string()
            } else {
                lines[location.line].to_string()
            };

            string.push_str(&format!(
                "\n\n{}{}{}",
                util::colors::FG_GRAY,
                result,
                util::colors::COLOR_RESET
            ));

            string.push_str(&format!(
                "\n{}{}{}{}",
                util::colors::FG_CYAN,
                " ".repeat(location.start),
                "^".repeat(location.end - location.start),
                util::colors::COLOR_RESET
            ));
        }

        return string;
    }

    pub fn visualise(&self) -> String {
        let mut file_contents: Result<String, String> = Err("<no file provided>".to_string());

        if let Some(ref location) = self.location {
            if let Some(ref file_name) = location.file_name {
                file_contents = match fs::read_to_string(file_name) {
                    Ok(ok) => Ok(ok),
                    Err(err) => Err(format!("<failed to read {}: {}>", file_name, err.kind())),
                }
            }

            return self._visualise(file_contents.unwrap_or("<no file provided>".to_string()));
        }

        return self._visualise("<no location>".to_string());
    }
}
