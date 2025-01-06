use std::sync::{Arc, Mutex};

use crate::{
    errors::{ErrorCode, ZephyrError},
    parser::nodes::{self, Node},
};

use super::{
    scope::{Scope, Variable},
    values::{self, RuntimeValue, RuntimeValueDetails},
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

    pub fn run_call(&mut self, expr: nodes::Call) -> R {
        let left = self.run(*expr.left.clone())?;

        let mut args: Vec<RuntimeValue> = vec![];
        for arg in expr.args {
            args.push(self.run(*arg)?);
        }

        match left {
            RuntimeValue::Function(func) => {
                let mut scope = Scope::new(Some(self.scope.clone()));
                for (i, v) in func.arguments.iter().enumerate() {
                    if i >= args.len() {
                        scope.insert(
                            v.clone(),
                            Variable::from(values::Null::new()),
                            Some(expr.location.clone()),
                        )?
                    } else {
                        scope.insert(
                            v.clone(),
                            Variable::from(args[i].clone()),
                            Some(expr.location.clone()),
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
