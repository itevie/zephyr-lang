use crate::{errors::ErrorCode, parser::nodes};

use super::{values, Interpreter, R};

impl Interpreter {
    pub fn run_while(&mut self, expr: nodes::WhileLoop) -> R {
        while self.run(*expr.test.clone())?.is_truthy() {
            match self.run(*expr.body.clone()) {
                Ok(_) => (),
                Err(err) => match err.code {
                    ErrorCode::Break => break,
                    ErrorCode::Continue => continue,
                    _ => return Err(err),
                },
            }
        }

        Ok(values::Null::new())
    }
}
