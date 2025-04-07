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
            DeclareType::Symbol(s) => self.scope.borrow_mut().insert(
                s.value,
                Variable {
                    is_const: expr.is_const,
                    value: value.clone(),
                },
                Some(s.location.clone()),
            )?,
            DeclareType::Array(a) => match value {
                RuntimeValue::Array(ref arr) => {
                    for (i, v) in a.iter().enumerate() {
                        if let Some(val) = arr.items.borrow().get(i) {
                            self.scope.borrow_mut().insert(
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
                x => {
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

        Ok(value.clone())
    }

    pub fn run_assign(&mut self, expr: nodes::Assign) -> R {
        let value = self.run(*expr.value)?;

        match *expr.assignee {
            Node::Symbol(ref symbol) => {
                self.scope.borrow_mut().modify(
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

        let proto = uuid::Uuid::new_v4().to_string();
        self.prototype_store.set(proto.clone(), Default::default());

        for (key, value) in expr.values {
            let val = values::ZString::new("".to_string());
            val.options.proto.borrow_mut().replace(proto.clone());
            val.options
                .tags
                .borrow_mut()
                .insert("__enum_base".to_string(), value.clone());
            items.insert(key.value, val.wrap());
        }

        let obj = values::Object::new(items).wrap();
        obj.options().proto.borrow_mut().replace(proto.clone());

        self.scope.borrow_mut().insert(
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
