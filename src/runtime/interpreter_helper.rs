use std::sync::{Arc, Mutex};

use crate::parser::nodes;

use super::{
    scope::Scope,
    values::{self, RuntimeValueUtils},
    Interpreter, R,
};

impl Interpreter {
    pub fn run_block(&mut self, expr: nodes::Block) -> R {
        let old_scope = self.swap_scope(Scope::new(Some(self.scope), self.scope.file_name()));

        let mut last_executed = values::Null::new().wrap();

        for i in expr.nodes {
            last_executed = self.run(*i)?;
        }

        self.swap_scope(old_scope);
        Ok(last_executed)
    }

    pub fn run_exported(&mut self, expr: nodes::ExportedBlock) -> R {
        let mut last_executed = values::Null::new().wrap();

        for i in expr.nodes {
            last_executed = self.run(*i)?;
        }

        Ok(last_executed)
    }
}
