use std::collections::HashMap;

use crate::{
    errors::{ErrorCode, ZephyrError},
    parser::nodes::{self, DeclareType, Node},
};

use super::{
    scope::Variable,
    values::{self, RuntimeValue, RuntimeValueUtils},
    Interpreter, R,
};

impl Interpreter {
    pub fn run_declare(&mut self, expr: nodes::Declare) -> R {
        let value = if let Some(e) = expr.value {
            self.run(*e)?
        } else {
            values::Null::new().wrap()
        };

        match expr.assignee {
            DeclareType::Symbol(s) => self.scope.lock().unwrap().insert(
                s.value,
                Variable {
                    is_const: expr.is_const,
                    value: value.clone(),
                },
                Some(s.location.clone()),
            )?,
            DeclareType::Array(a) => match value.as_ref_tuple()? {
                (RuntimeValue::Array(arr), Some(_)) => {
                    for (i, v) in a.iter().enumerate() {
                        if let Some(val) = arr.items.get(i) {
                            self.scope.lock().unwrap().insert(
                                v.value.clone(),
                                Variable {
                                    is_const: expr.is_const,
                                    value: val.clone(),
                                },
                                Some(v.location.clone()),
                            )?;
                        } else {
                            return Err(ZephyrError {
                                message: "Out of bounds".to_string(),
                                code: ErrorCode::OutOfBounds,
                                location: Some(v.location.clone()),
                            });
                        }
                    }
                }
                (x, _) => {
                    return Err(ZephyrError {
                        message: format!(
                            "Cannot assign to a array declaration with a {}",
                            x.type_name()
                        ),
                        code: ErrorCode::TypeError,
                        location: Some(expr.location.clone()),
                    })
                }
            },
            _ => panic!(),
        }

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
            Node::Member(member) => return self.run_member(member, Some(value)),
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

    pub fn run_enum(&mut self, expr: nodes::Enum) -> R {
        let mut items: HashMap<String, RuntimeValue> = HashMap::new();

        let proto = values::Object::new(HashMap::new())
            .as_ref_val()
            .as_ref_tuple()?
            .1
            .unwrap()
            .location
            .as_basic()
            .unwrap();

        for (key, value) in expr.values {
            let val = values::ZString::new("".to_string());
            val.options.proto.lock().unwrap().replace(proto);
            val.options
                .tags
                .lock()
                .unwrap()
                .insert("__enum_base".to_string(), value.clone());
            items.insert(key.value, val.wrap());
        }

        let obj = values::Object::new(items).as_ref_val();
        obj.options().proto.lock().unwrap().replace(proto);

        self.scope.lock().unwrap().insert(
            expr.name.value,
            Variable {
                is_const: true,
                value: obj,
            },
            Some(expr.name.location.clone()),
        )?;

        Ok(values::Null::new().wrap())
    }
}
