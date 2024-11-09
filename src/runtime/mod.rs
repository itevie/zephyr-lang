use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use scope::{Scope, Variable};
use values::RuntimeValue;

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::{self, TokenType},
    parser::nodes::{self, Node},
};

pub mod memory_store;
pub mod scope;
pub mod values;

type R = Result<RuntimeValue, ZephyrError>;

pub struct Interpreter {
    pub scope: Arc<Mutex<Scope>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            scope: Arc::from(Mutex::from(Scope::new(None))),
        }
    }

    pub fn swap_scope(&mut self, scope: Arc<Mutex<Scope>>) -> Arc<Mutex<Scope>> {
        std::mem::replace(&mut self.scope, scope)
    }

    pub fn run(&mut self, node: Node) -> R {
        match node {
            Node::Block(expr) => {
                let old_scope = self.swap_scope(Arc::from(Mutex::from(Scope::new(Some(
                    Arc::clone(&self.scope),
                )))));

                let mut last_executed = values::Null::new();

                for i in expr.nodes {
                    last_executed = self.run(*i)?;
                }

                self.swap_scope(old_scope);
                Ok(last_executed)
            }

            Node::Arithmetic(expr) => {
                let left = self.run(*expr.left)?;
                let right = self.run(*expr.right)?;

                if let (RuntimeValue::Number(left_number), RuntimeValue::Number(right_number)) =
                    (&left, &right)
                {
                    return Ok(values::Number::new(match expr.t {
                        TokenType::Additive(tokens::Additive::Plus) => {
                            left_number.value + right_number.value
                        }
                        TokenType::Additive(tokens::Additive::Minus) => {
                            left_number.value - right_number.value
                        }
                        TokenType::Multiplicative(tokens::Multiplicative::Divide) => {
                            left_number.value / right_number.value
                        }
                        TokenType::Multiplicative(tokens::Multiplicative::Multiply) => {
                            left_number.value / right_number.value
                        }
                        TokenType::Multiplicative(tokens::Multiplicative::Modulo) => {
                            left_number.value % right_number.value
                        }
                        _ => unreachable!(),
                    }));
                }

                let result = match left {
                    // string ? *
                    RuntimeValue::ZString(ref left_value) => match expr.t {
                        // string + *
                        TokenType::Additive(tokens::Additive::Plus) => Some(values::ZString::new(
                            left_value.value.clone() + &right.to_string()?,
                        )),
                        _ => None,
                    },
                    _ => None,
                };

                match result {
                    Some(ok) => Ok(ok),
                    None => Err(ZephyrError {
                        code: ErrorCode::InvalidOperation,
                        message: format!(
                            "Cannot handle {} {:?} {}",
                            left.type_name(),
                            expr.t,
                            right.type_name()
                        ),
                        location: Some(expr.location),
                    }),
                }
            }

            Node::Declare(expr) => {
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
                )?;

                Ok(value)
            }

            Node::Function(expr) => Ok(RuntimeValue::Function(values::Function {
                body: expr.body,
                name: expr.name.map(|x| x.value),
            })),

            Node::Array(expr) => {
                let mut items: Vec<RuntimeValue> = vec![];
                for i in expr.items {
                    items.push(self.run(*i)?);
                }
                Ok(values::Array::new_ref(items))
            }
            Node::Object(expr) => {
                let mut items: HashMap<String, RuntimeValue> = HashMap::new();

                for (k, v) in expr.items {
                    items.insert(k, self.run(*v)?);
                }

                Ok(values::Object::new_ref(items))
            }

            Node::Assign(expr) => {
                let value = self.run(*expr.value)?;

                match *expr.assignee {
                    Node::Symbol(symbol) => {
                        self.scope
                            .lock()
                            .unwrap()
                            .modify(symbol.value, value.clone())?;
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

            Node::Call(expr) => {
                let left = self.run(*expr.left.clone())?;

                match left {
                    RuntimeValue::Function(func) => {
                        return self.run(Node::Block(func.body));
                    }
                    _ => {
                        return Err(ZephyrError {
                            code: ErrorCode::InvalidOperation,
                            message: format!("Cannot call a {}", left.type_name()),
                            location: Some(expr.left.location().clone()),
                        })
                    }
                }
            }

            Node::Member(expr) => self.member(expr),

            Node::Number(expr) => Ok(values::Number::new(expr.value)),
            Node::ZString(expr) => Ok(values::ZString::new(expr.value)),
            Node::Symbol(expr) => Ok(self.scope.lock().unwrap().lookup(expr.value)?.clone()),
        }
    }

    pub fn member(&mut self, expr: nodes::Member) -> R {
        let left = self.run(*expr.left.clone())?.check_ref()?;

        if !expr.computed {
            let key = match *expr.right {
                Node::Symbol(sym) => sym.value,
                _ => unreachable!(),
            };

            todo!();
        } else {
            let right = self.run(*expr.right.clone())?.check_ref()?;

            match left {
                // object[_]
                (RuntimeValue::Object(obj), Some(_)) => match right {
                    // object[string]
                    (RuntimeValue::ZString(string), None) => {
                        if !obj.items.contains_key(&string.value) {
                            return Err(ZephyrError {
                                code: ErrorCode::InvalidKey,
                                message: format!("Object does not contain key {}", string.value),
                                location: Some(expr.right.location().clone()),
                            });
                        }

                        Ok(obj.items.get(&string.value).unwrap().clone())
                    }
                    _ => {
                        return Err(ZephyrError {
                            code: ErrorCode::InvalidOperation,
                            message: format!(
                                "Cannot access an object with a {}",
                                right.0.type_name()
                            ),
                            location: Some(expr.right.location().clone()),
                        })
                    }
                },
                // array[_]
                (RuntimeValue::Array(arr), Some(_)) => match right {
                    // array[number]
                    (RuntimeValue::Number(num), None) => {
                        // Check out of bounds
                        if num.value as usize >= arr.items.len() {
                            return Err(ZephyrError {
                                code: ErrorCode::OutOfBounds,
                                message: format!(
                                    "Array length is {}, but index wanted was {}",
                                    arr.items.len(),
                                    num.value
                                ),
                                location: Some(expr.location),
                            });
                        }

                        Ok(arr.items[num.value as usize].clone())
                    }
                    // array[array]
                    (RuntimeValue::Array(key_arr), Some(_)) => {
                        let mut items: Vec<RuntimeValue> = vec![];

                        for (index, i) in key_arr.items.iter().enumerate() {
                            match i {
                                RuntimeValue::Number(num) => items.push({
                                    // Check out of bounds
                                    if num.value as usize >= arr.items.len() {
                                        return Err(ZephyrError {
                                            code: ErrorCode::OutOfBounds,
                                            message: format!(
                                                "Array length is {}, but index wanted was {} at index {}",
                                                arr.items.len(),
                                                num.value,
                                                index
                                            ),
                                            location: Some(expr.location),
                                        });
                                    }

                                    arr.items[num.value as usize].clone()
                                }),
                                _ => return Err(ZephyrError {
                                    code: ErrorCode::InvalidOperation,
                                    message: format!(
                                        "All elements in array key must be a number, but got {} at index {}", 
                                        i.type_name(),
                                        index
                                    ),
                                    location: None,
                                })
                            }
                        }

                        Ok(values::Array::new_ref(items))
                    }
                    _ => {
                        return Err(ZephyrError {
                            code: ErrorCode::InvalidOperation,
                            message: format!(
                                "Cannot access an array with a {}",
                                right.0.type_name()
                            ),
                            location: Some(expr.right.location().clone()),
                        })
                    }
                },
                _ => {
                    return Err(ZephyrError {
                        code: ErrorCode::InvalidOperation,
                        message: format!("Cannot access a {}", left.0.type_name()),
                        location: Some(expr.left.location().clone()),
                    })
                }
            }
        }
    }
}
