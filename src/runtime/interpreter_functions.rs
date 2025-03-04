use std::sync::{Arc, Mutex};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::Location,
    parser::nodes::{self, Node},
};

use super::{
    native::NativeExecutionContext,
    scope::{Scope, Variable},
    values::{self, FunctionType, RuntimeValue, RuntimeValueDetails},
    Interpreter, R,
};

impl Interpreter {
    pub fn run_make_function(&mut self, expr: nodes::Function) -> R {
        Ok(RuntimeValue::Function(values::Function {
            options: RuntimeValueDetails::default(),
            body: expr.body,
            name: expr.name.map(|x| x.value),
            scope: self.scope.clone(),
            arguments: expr.args.iter().map(|x| x.value.clone()).collect(),
        }))
    }

    pub fn run_function(
        &mut self,
        func: FunctionType,
        args: Vec<RuntimeValue>,
        location: Location,
    ) -> R {
        match func {
            FunctionType::Function(func) => {
                let mut scope = Scope::new_from_parent(func.scope.clone());
                for (i, v) in func.arguments.iter().enumerate() {
                    if i >= args.len() {
                        scope.insert(
                            v.clone(),
                            Variable::from(values::Null::new()),
                            Some(location.clone()),
                        )?
                    } else {
                        scope.insert(
                            v.clone(),
                            Variable::from(args[i].clone()),
                            Some(location.clone()),
                        )?
                    }
                }

                let old = self.swap_scope(Arc::from(Mutex::from(scope)));
                let result = self.run(Node::Block(func.body));
                self.swap_scope(old);

                if let Err(err) = &result {
                    if let ErrorCode::Return(Some(val)) = &err.code {
                        return Ok(val.clone());
                    } else if let ErrorCode::Return(None) = &err.code {
                        return Ok(values::Null::new());
                    }
                }

                return result;
            }
            FunctionType::NativeFunction(func) => {
                let ctx = NativeExecutionContext {
                    args,
                    interpreter: self.clone(),
                    location: location.clone(),
                    file_name: self.scope.lock().unwrap().file_name.clone(),
                };

                (func.func)(ctx)
            }
        }
    }

    pub fn run_call(&mut self, expr: nodes::Call) -> R {
        let left = self.run(*expr.left.clone())?;

        let mut args: Vec<RuntimeValue> = vec![];
        for arg in expr.args {
            args.push(self.run(*arg)?);
        }

        if let Some(val) = &left.options().proto_value {
            args.insert(0, *val.clone());
        }

        let left_clone = left.clone();
        let tag_lock = left_clone.options().tags.lock().unwrap();
        if let Some(enum_id) = tag_lock.get("__enum_base").cloned() {
            if args.len() > 1 {
                return Err(ZephyrError {
                    code: ErrorCode::TypeError,
                    message: "Expected 1 or 0 arguments for enum variant".to_string(),
                    location: Some(expr.location.clone()),
                });
            }

            let null_value = values::Null::new();
            let value = args.get(0).unwrap_or(&null_value);
            value
                .options()
                .tags
                .lock()
                .unwrap()
                .insert("__enum_variant".to_string(), enum_id.clone());
            return Ok(value.clone());
        }

        match left {
            RuntimeValue::Function(func) => {
                self.run_function(FunctionType::Function(func), args, expr.location)
            }
            RuntimeValue::NativeFunction(func) => {
                self.run_function(FunctionType::NativeFunction(func), args, expr.location)
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
}
