use std::collections::HashMap;

use crate::{
    errors::{ErrorCode, ZephyrError},
    parser::nodes::{self, Node},
};

use super::{
    values::{self, RuntimeValue, RuntimeValueDetails, RuntimeValueUtils},
    Interpreter, R,
};

impl Interpreter {
    pub fn run_member(&mut self, expr: nodes::Member, set: Option<RuntimeValue>) -> R {
        let left = self.run(*expr.left.clone())?;

        // Check if it is being accessed via x.a
        if !expr.computed {
            let key = match *expr.right {
                Node::Symbol(sym) => sym.value,
                _ => {
                    return Err(ZephyrError {
                        message: "Expected a symbol".to_string(),
                        code: ErrorCode::TypeError,
                        location: Some(expr.location),
                    })
                }
            };

            return self.member_check_basic(left.clone(), key, set);
        } else {
            let right = self.run(*expr.right.clone())?;

            return match right {
                RuntimeValue::ZString(string) => {
                    self.member_check_basic(left.clone(), string.value, set)
                }
                RuntimeValue::RangeValue(_range) => {
                    let mut range = _range.clone();
                    let iter = left.iter()?;

                    if range.start < 0f64 {
                        range.start = iter.len() as f64 + range.start;
                    }

                    if range.end < 0f64 {
                        range.end = iter.len() as f64 + range.end;
                    }

                    let indexes = range
                        .iter_f64()?
                        .iter()
                        .map(|x| *x as usize)
                        .collect::<Vec<usize>>();
                    let mut parts: Vec<RuntimeValue> = vec![];

                    for index in indexes {
                        if let Some(val) = iter.get(index) {
                            parts.push(val.clone());
                        } else {
                            return Err(ZephyrError {
                                message: "Out of bounds".to_string(),
                                code: ErrorCode::OutOfBounds,
                                location: Some(expr.location),
                            });
                        }
                    }

                    Ok(match left {
                        /*RuntimeValue::ZString(_) => values::ZString::new(
                            parts
                                .iter()
                                .map(|z| match z {
                                    RuntimeValue::ZString(a) => a.value.clone(),
                                    _ => unreachable!(),
                                })
                                .collect::<String>(),
                        )
                        .wrap(),*/
                        _ => values::Array::new(parts).wrap(),
                    })
                }
                RuntimeValue::Number(number) => {
                    if let Some(set) = set {
                        match left {
                            RuntimeValue::Array(ref arr) => {
                                let mut borrow = arr.items.borrow_mut();
                                if number.value as usize > borrow.len() {
                                    return Err(ZephyrError { code: ErrorCode::OutOfBounds, message: format!("Trying to assign at index {} but array is only {} items long", number.value, borrow.len()), location: Some(expr.location) });
                                } else {
                                    if number.value as usize == borrow.len() {
                                        borrow.push(set);
                                    } else {
                                        borrow[number.value as usize] = set;
                                    }

                                    return Ok(values::Null::new().wrap());
                                }
                            }
                            _ => {
                                return Err(ZephyrError {
                                    code: ErrorCode::InvalidOperation,
                                    message: format!("Cannot assign to a {}", left.type_name()),
                                    location: Some(expr.location),
                                })
                            }
                        }
                    }

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
            return match value.options().proto.borrow().as_ref() {
                Some(proto) => Ok(self.prototype_store.get(proto).wrap()),
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
                    .borrow()
                    .iter()
                    .map(|v| (v.0.clone(), values::ZString::new(v.1.clone()).wrap()))
                    .collect::<HashMap<String, RuntimeValue>>(),
            )
            .wrap());
        }

        match value {
            RuntimeValue::Object(ref obj) => {
                if let Some(setter) = set {
                    if obj.items.borrow().contains_key(&key) {
                        obj.items.borrow_mut().remove(&key);
                    }

                    obj.items.borrow_mut().insert(key, setter);

                    return Ok(values::Null::new().wrap());
                } else if let Some(val) = obj.items.borrow().get(&key) {
                    return Ok(val.clone());
                }
            }
            _ => (),
        }

        if let Some(proto_ref) = value.options().proto.borrow().as_ref() {
            let mut proto_ref = Some(proto_ref.clone());

            while let Some(ref proto) = proto_ref {
                let prototype = self.prototype_store.get(proto.clone());

                let borrow = prototype.items.borrow_mut();
                if let Some(proto_value) = borrow.get(&key) {
                    let mut new_value = proto_value.clone();
                    new_value.set_options(RuntimeValueDetails {
                        proto_value: Some(Box::from(value.clone())),
                        ..proto_value.options().clone()
                    });

                    return Ok(new_value.clone());
                } else if let Some(new_proto) = prototype.options.proto.borrow().clone() {
                    if new_proto == proto_ref.unwrap() {
                        break;
                    }
                    proto_ref = Some(new_proto)
                } else {
                    break;
                }
            }
        }

        if let Some(val) = self.prototype_store.get("any").items.borrow().get(&key) {
            return Ok(val.clone());
        }

        Err(ZephyrError {
            message: format!("Object does not define property {}", key),
            code: ErrorCode::InvalidProperty,
            location: None,
        })
    }
}
