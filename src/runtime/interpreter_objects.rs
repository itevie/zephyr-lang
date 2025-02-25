use std::collections::HashMap;

use crate::{
    errors::{ErrorCode, ZephyrError},
    parser::nodes::{self, Node},
};

use super::{
    memory_store::{self, store_get},
    values::{self, ReferenceType, RuntimeValue, RuntimeValueDetails},
    Interpreter, R,
};

impl Interpreter {
    pub fn run_member(&mut self, expr: nodes::Member, set: Option<RuntimeValue>) -> R {
        let left = self.run(*expr.left.clone())?;

        // Check if it is being accessed via x.a
        if !expr.computed {
            let key = match *expr.right {
                Node::Symbol(sym) => sym.value,
                _ => unreachable!(),
            };

            return self.member_check_basic(left.clone(), key, set);
        } else {
            let right = self.run(*expr.right.clone())?.as_ref_tuple()?;

            return match right.0 {
                RuntimeValue::ZString(string) => {
                    self.member_check_basic(left.clone(), string.value, set)
                }
                RuntimeValue::Number(number) => {
                    let iter = left.iter()?;

                    if let Some(val) = iter.get(number.value as usize) {
                        return Ok(val.clone());
                    } else {
                        return Err(ZephyrError {
                            message: "Out of bounds".to_string(),
                            code: ErrorCode::OutOfBounds,
                            location: Some(expr.location),
                        });
                    }
                }
                x => Err(ZephyrError {
                    message: format!("Cannot access {} via {}", left.type_name(), x.type_name()),
                    code: ErrorCode::TypeError,
                    location: Some(expr.location),
                }),
            };
        }
    }

    pub fn member_check_basic(
        &mut self,
        value: RuntimeValue,
        key: String,
        set: Option<RuntimeValue>,
    ) -> R {
        // Prescedence:
        // - __proto check
        // - __tag check
        // - object property check
        // - property chain check

        if &key == "__proto" {
            return match value.options().proto {
                Some(proto) => Ok(values::Reference::new(proto)),
                None => Err(ZephyrError {
                    message: "The value does not have a prototype".to_string(),
                    code: ErrorCode::UnknownReference,
                    location: None,
                }),
            };
        }

        if &key == "__tags" {
            if let Some(_) = set {
                return Err(ZephyrError {
                    message: "Cannot assign to a value's __tags".to_string(),
                    code: ErrorCode::InvalidOperation,
                    location: None,
                });
            }

            return Ok(values::Object::new(
                value
                    .options()
                    .tags
                    .lock()
                    .unwrap()
                    .clone()
                    .iter()
                    .map(|v| (v.0.clone(), values::ZString::new(v.1.clone())))
                    .collect::<HashMap<String, RuntimeValue>>(),
            ));
        }

        match value.as_ref_tuple()? {
            (RuntimeValue::Object(mut obj), Some(r)) => {
                if let Some(setter) = set {
                    if obj.items.contains_key(&key) {
                        obj.items.remove(&key);
                    }

                    obj.items.insert(key, setter);
                    memory_store::store_set(
                        match r.location {
                            ReferenceType::Basic(i) => i,
                            _ => panic!(),
                        },
                        RuntimeValue::Object(obj),
                    );

                    return Ok(values::Null::new());
                } else if let Some(val) = obj.items.get(&key) {
                    return Ok(val.clone());
                }
            }
            _ => (),
        }

        if let Some(proto_ref) = value.options().proto {
            let prototype = match store_get(proto_ref) {
                RuntimeValue::Object(o) => o,
                _ => panic!("Expected an object as the prototype."),
            };

            if let Some(proto_value) = prototype.items.get(&key) {
                let mut new_value = proto_value.clone();
                new_value.set_options(RuntimeValueDetails {
                    proto_value: Some(Box::from(value.clone())),
                    ..proto_value.options().clone()
                });

                return Ok(new_value.clone());
            }
        }

        Err(ZephyrError {
            message: format!("Object does not define property {}", key),
            code: ErrorCode::InvalidProperty,
            location: None,
        })
    }
}
