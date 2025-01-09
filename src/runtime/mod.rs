use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use scope::Scope;
use values::{Null, RuntimeValue};

use crate::{
    errors::{ErrorCode, ZephyrError},
    parser::nodes::{self, InterruptType, Node},
};

pub mod interpreter_conditionals;
pub mod interpreter_functions;
pub mod interpreter_helper;
pub mod interpreter_loops;
pub mod interpreter_objects;
pub mod interpreter_operators;
pub mod interpreter_variables;
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
        match node.clone() {
            // ----- conditionals -----
            Node::If(expr) => self.run_if(expr),
            Node::Match(expr) => self.run_match(expr),

            // ----- functions -----
            Node::Function(expr) => self.run_make_function(expr),
            Node::Call(expr) => self.run_call(expr),

            // ----- helpers -----
            Node::Block(expr) => self.run_block(expr),

            // ----- loops -----
            Node::WhileLoop(expr) => self.run_while(expr),

            // ----- operators -----
            Node::Arithmetic(expr) => self.run_arithmetic(expr),
            Node::Comp(expr) => self.run_comp(expr),

            // ----- variables -----
            Node::Declare(expr) => self.run_declare(expr),
            Node::Assign(expr) => self.run_assign(expr),

            // ----- other -----
            Node::Export(expr) => {
                todo!()
            }

            Node::Interrupt(expr) => match expr.t {
                InterruptType::Continue => Err(ZephyrError {
                    message: "Cannot continue here".to_string(),
                    code: ErrorCode::Continue,
                    location: Some(expr.location.clone()),
                }),
                InterruptType::Break => Err(ZephyrError {
                    message: "Cannot break here".to_string(),
                    code: ErrorCode::Break,
                    location: Some(expr.location.clone()),
                }),
                InterruptType::Return(val) => {
                    let value = if let Some(v) = val {
                        Some(self.run(*v)?)
                    } else {
                        None
                    };

                    Err(ZephyrError {
                        message: "Cannot return here".to_string(),
                        code: ErrorCode::Return(value),
                        location: Some(expr.location.clone()),
                    })
                }
            },

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
                    items.insert(k, self.run(*v.value)?);
                }

                Ok(values::Object::new_ref(items))
            }

            Node::Member(expr) => self.run_member(expr, None),

            Node::Number(expr) => Ok(values::Number::new(expr.value)),
            Node::ZString(expr) => Ok(values::ZString::new(expr.value)),
            Node::Symbol(expr) => Ok(self
                .scope
                .lock()
                .unwrap()
                .lookup(expr.value, Some(expr.location))?
                .clone()),

            Node::DebugNode(expr) => {
                let result = self.run(*expr.node)?;
                println!("{:#?}", result);
                return Ok(Null::new());
            }
        }
        .map_err(|ref x| {
            let mut err = x.clone();
            if let None = x.location {
                err.location = Some(node.location().clone())
            }
            err
        })
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
