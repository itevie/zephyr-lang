use crate::{
    errors::{ErrorCode, ZephyrError},
    parser::nodes,
};

use super::{values, Interpreter, R};

impl Interpreter {
    pub fn run_encapsulate_error(&mut self, expr: nodes::EncapsulateError) -> R {
        let result = self.run(*expr.left);

        Ok(match result {
            Ok(val) => {
                val.options().tags.lock().unwrap().insert(
                    "__enum_variant".to_string(),
                    "Result.Ok__Zephyr".to_string(),
                );
                val
            }
            Err(err) => {
                let string = values::ZString::new(err.message);
                string.options().tags.lock().unwrap().insert(
                    "__enum_variant".to_string(),
                    "Result.Err__Zephyr".to_string(),
                );
                string
            }
        })
    }

    pub fn run_propogate_error(&mut self, expr: nodes::PropogateError) -> R {
        let result = self.run(*expr.left)?;

        let mut lock = result.options().tags.lock().unwrap();
        if let Some(tag) = lock.get("__enum_variant").cloned() {
            if tag == "Result.Err__Zephyr" {
                return Err(ZephyrError {
                    message: "Cannot return here".to_string(),
                    code: ErrorCode::Return(Some(result.clone())),
                    location: Some(expr.location.clone()),
                });
            } else {
                lock.remove("__enum_variant");
                return Ok(result.clone());
            }
        } else {
            return Err(ZephyrError {
                message: "Cannot propogate error from non-error value".to_string(),
                code: ErrorCode::TypeError,
                location: Some(expr.location.clone()),
            });
        }
    }
}
