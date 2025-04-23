use std::collections::HashMap;

use crate::{
    errors::{ErrorCode, ZephyrError},
    parser::nodes,
};

use super::{
    values::{self, RuntimeValue, RuntimeValueDetails, RuntimeValueUtils},
    Interpreter, R,
};

impl Interpreter {
    pub fn run_array(&mut self, expr: nodes::Array) -> R {
        let mut items: Vec<RuntimeValue> = vec![];
        for i in expr.items {
            items.push(self.run(i)?);
        }
        Ok(values::Array::new(items).wrap())
    }

    pub fn run_object(&mut self, expr: nodes::Object) -> R {
        let mut items: HashMap<String, RuntimeValue> = HashMap::new();

        for (k, v) in expr.items {
            items.insert(k, self.run(*v.value)?);
        }

        Ok(values::Object::new(items).wrap())
    }

    pub fn run_range(&mut self, expr: nodes::Range) -> R {
        let start = match self.run(*expr.start.clone())? {
            RuntimeValue::Number(n) => n.value,
            _ => {
                return Err(ZephyrError {
                    message: "Expected number for start of range".to_string(),
                    code: ErrorCode::TypeError,
                    location: Some(expr.start.location().clone()),
                })
            }
        };
        let end = match self.run(*expr.end.clone())? {
            RuntimeValue::Number(n) => n.value,
            _ => {
                return Err(ZephyrError {
                    message: "Expected number for end of range".to_string(),
                    code: ErrorCode::TypeError,
                    location: Some(expr.end.location().clone()),
                })
            }
        };
        let step = match expr.step {
            Some(v) => match self.run(*v.clone())? {
                RuntimeValue::Number(n) => Some(n.value),
                _ => {
                    return Err(ZephyrError {
                        message: "Expected number for step of range".to_string(),
                        code: ErrorCode::TypeError,
                        location: Some(v.location().clone()),
                    })
                }
            },
            None => None,
        };

        Ok(RuntimeValue::RangeValue(values::RangeValue {
            options: RuntimeValueDetails::default(),
            start,
            end,
            step,
            inclusive_end: expr.inclusive_end,
        }))
    }
}
