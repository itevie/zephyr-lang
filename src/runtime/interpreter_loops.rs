use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::{errors::ErrorCode, parser::nodes};

use super::{
    insert_node_timing,
    scope::{Scope, Variable},
    time_this,
    values::{self, RuntimeValueUtils},
    Interpreter, R,
};

impl Interpreter {
    pub fn run_while(&mut self, expr: nodes::WhileLoop) -> R {
        while self.run(*expr.test.clone())?.is_truthy() {
            match self.run(*expr.body.clone()) {
                Ok(_) => (),
                Err(err) => match err.code {
                    ErrorCode::Break => break,
                    ErrorCode::Continue => continue,
                    _ => return Err(err),
                },
            }
        }

        Ok(values::Null::new().wrap())
    }

    pub fn run_for(&mut self, expr: nodes::For) -> R {
        let values = self.run(*expr.iterator)?.iter()?;

        for (i, v) in values.iter().enumerate() {
            let scope: Scope;
            time_this!("Mini:ForCreateScope".to_string(), {
                scope = Scope::new(Some(self.scope), self.scope.file_name());
                scope.insert(
                    expr.index_symbol.value.clone(),
                    Variable::new(values::Number::new(i as f64).wrap()),
                    Some(expr.index_symbol.location.clone()),
                )?;

                if let Some(ref x) = expr.value_symbol {
                    scope.insert(
                        x.value.clone(),
                        Variable::new(v.clone()),
                        Some(x.location.clone()),
                    )?;
                }
            });

            time_this!("Mini:ForRun".to_string(), {
                let old_scope = self.swap_scope(scope);
                self.run(*expr.block.clone())?;
                self.swap_scope(old_scope);
            });
        }

        Ok(values::Null::new().wrap())
    }
}
