use crate::{
    errors::{ErrorCode, ZephyrError},
    parser::nodes::{self, Node},
};

use super::{
    scope::Variable,
    values::{self},
    Interpreter, R,
};

impl Interpreter {
    pub fn run_declare(&mut self, expr: nodes::Declare) -> R {
        let value = if let Some(e) = expr.value {
            self.run(*e)?
        } else {
            values::Null::new()
        };

        self.scope.lock().unwrap().insert(
            expr.symbol.value,
            Variable {
                is_const: expr.is_const,
                value: value.clone(),
            },
            Some(expr.location),
        )?;

        Ok(value)
    }

    pub fn run_assign(&mut self, expr: nodes::Assign) -> R {
        let value = self.run(*expr.value)?;

        match *expr.assignee {
            Node::Symbol(ref symbol) => {
                self.scope.lock().unwrap().modify(
                    symbol.value.clone(),
                    value.clone(),
                    Some(expr.assignee.location().clone()),
                )?;
            }
            x => {
                return Err(ZephyrError {
                    code: ErrorCode::InvalidOperation,
                    message: format!("Cannot assign to a {:?}", x),
                    location: Some(x.location().clone()),
                })
            }
        }

        Ok(value)
    }
}
